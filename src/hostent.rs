use std::ffi::CStr;
use std::fmt;
use std::marker::PhantomData;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::os::raw::{c_char, c_int};
use std::slice;

use itertools::Itertools;

use crate::types::AddressFamily;
use crate::utils::address_family;

fn hostname(hostent: &libc::hostent) -> &CStr {
    unsafe { CStr::from_ptr(hostent.h_name) }
}

fn addresses(hostent: &libc::hostent) -> HostAddressResultsIter {
    // h_addrtype is `c_short` on windows, `c_int` on unix.  Tell clippy to allow the identity
    // conversion in the latter case.
    #[allow(clippy::useless_conversion)]
    let addrtype = c_int::from(hostent.h_addrtype);
    HostAddressResultsIter {
        family: address_family(addrtype),
        next: unsafe { &*(hostent.h_addr_list as *const _) },
    }
}

fn aliases(hostent: &libc::hostent) -> HostAliasResultsIter {
    HostAliasResultsIter {
        next: unsafe { &*(hostent.h_aliases as *const _) },
    }
}

fn display(hostent: &libc::hostent, fmt: &mut fmt::Formatter) -> fmt::Result {
    write!(
        fmt,
        "Hostname: {}, ",
        hostname(hostent).to_str().unwrap_or("<not utf8>")
    )?;
    let addresses = addresses(hostent).format(", ");
    write!(fmt, "Addresses: [{addresses}]")?;
    let aliases = aliases(hostent)
        .map(|cstr| cstr.to_str().unwrap_or("<not utf8>"))
        .format(", ");
    write!(fmt, "Aliases: [{aliases}]")
}

pub trait HasHostent<'a>: Sized {
    fn hostent(self) -> &'a libc::hostent;

    fn hostname(self) -> &'a CStr {
        let hostent = self.hostent();
        hostname(hostent)
    }

    fn addresses(self) -> HostAddressResultsIter<'a> {
        let hostent = self.hostent();
        addresses(hostent)
    }

    fn aliases(self) -> HostAliasResultsIter<'a> {
        let hostent = self.hostent();
        aliases(hostent)
    }
}

#[derive(Debug)]
pub struct HostentOwned {
    inner: *mut libc::hostent,
    phantom: PhantomData<libc::hostent>,
}

impl HostentOwned {
    pub fn new(hostent: *mut libc::hostent) -> Self {
        HostentOwned {
            inner: hostent,
            phantom: PhantomData,
        }
    }
}

impl<'a> HasHostent<'a> for &'a HostentOwned {
    fn hostent(self) -> &'a libc::hostent {
        unsafe { &*self.inner }
    }
}

impl fmt::Display for HostentOwned {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let hostent = self.hostent();
        display(hostent, fmt)
    }
}

impl Drop for HostentOwned {
    fn drop(&mut self) {
        unsafe {
            c_ares_sys::ares_free_hostent(self.inner);
        }
    }
}

unsafe impl Send for HostentOwned {}
unsafe impl Sync for HostentOwned {}

#[derive(Clone, Copy)]
pub struct HostentBorrowed<'a> {
    inner: &'a libc::hostent,
}

impl<'a> HostentBorrowed<'a> {
    pub fn new(hostent: &'a libc::hostent) -> Self {
        HostentBorrowed { inner: hostent }
    }
}

impl<'a> HasHostent<'a> for HostentBorrowed<'a> {
    fn hostent(self) -> &'a libc::hostent {
        self.inner
    }
}

impl<'a> fmt::Display for HostentBorrowed<'a> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let hostent = self.hostent();
        display(hostent, fmt)
    }
}

unsafe impl<'a> Send for HostentBorrowed<'a> {}
unsafe impl<'a> Sync for HostentBorrowed<'a> {}

// Get an IpAddr from a family and an array of bytes, as found in a `hostent`.
unsafe fn ip_address_from_bytes(family: AddressFamily, h_addr: *const u8) -> Option<IpAddr> {
    match family {
        AddressFamily::INET => {
            let source = slice::from_raw_parts(h_addr, 4);
            let bytes: [u8; 4] = source.try_into().unwrap();
            let ipv4 = Ipv4Addr::from(bytes);
            Some(IpAddr::V4(ipv4))
        }
        AddressFamily::INET6 => {
            let source = slice::from_raw_parts(h_addr, 16);
            let bytes: [u8; 16] = source.try_into().unwrap();
            let ipv6 = Ipv6Addr::from(bytes);
            Some(IpAddr::V6(ipv6))
        }
        _ => None,
    }
}

/// Iterator of `IpAddr`s.
#[derive(Clone, Copy, Debug)]
pub struct HostAddressResultsIter<'a> {
    family: Option<AddressFamily>,
    next: &'a *const c_char,
}

impl<'a> Iterator for HostAddressResultsIter<'a> {
    type Item = IpAddr;
    fn next(&mut self) -> Option<Self::Item> {
        let h_addr = *self.next;
        if h_addr.is_null() {
            None
        } else {
            unsafe {
                self.next = &*(self.next as *const *const c_char).offset(1);
                self.family
                    .and_then(|family| ip_address_from_bytes(family, h_addr as *const u8))
            }
        }
    }
}

unsafe impl<'a> Send for HostAddressResultsIter<'a> {}
unsafe impl<'a> Sync for HostAddressResultsIter<'a> {}

/// Iterator of `&'a CStr`s.
///
/// Each item is very likely to be a valid UTF-8 string, but the underlying `c-ares` library does
/// not guarantee this - so we leave it to users to decide whether they prefer a fallible
/// conversion, a lossy conversion, or something else altogether.
#[derive(Clone, Copy, Debug)]
pub struct HostAliasResultsIter<'a> {
    next: &'a *const c_char,
}

impl<'a> Iterator for HostAliasResultsIter<'a> {
    type Item = &'a CStr;
    fn next(&mut self) -> Option<Self::Item> {
        let h_alias = *self.next;
        if h_alias.is_null() {
            None
        } else {
            unsafe {
                self.next = &*(self.next as *const *const c_char).offset(1);
                let c_str = CStr::from_ptr(h_alias);
                Some(c_str)
            }
        }
    }
}

unsafe impl<'a> Send for HostAliasResultsIter<'a> {}
unsafe impl<'a> Sync for HostAliasResultsIter<'a> {}
