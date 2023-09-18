#[cfg(not(feature = "vendored"))]
fn main() {
    pkg_config::Config::new()
        .atleast_version("1.12.0")
        .probe("libcares")
        .unwrap();
}

#[cfg(feature = "vendored")]
fn main() {
    c_ares_src::build();
}
