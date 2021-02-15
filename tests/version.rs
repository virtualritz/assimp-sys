extern crate assimp_sys;
use assimp_sys::*;

#[test]
fn check_version() {
    let major = unsafe { aiGetVersionMajor() };
    let minor = unsafe { aiGetVersionMinor() };
    assert_eq!(major, 5);
    assert_eq!(minor, 0);
}
