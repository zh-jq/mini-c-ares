fn main() {
    pkg_config::probe_library("libcares").unwrap();
}
