
#[cfg(not(features = "vendored"))]
fn main() {
    pkg_config::Config::new()
        .atleast_version("1.12.0")
        .probe("libcares")
        .unwrap();
}

#[cfg(features = "vendored")]
fn main() {
    c_ares_src::build();
}

