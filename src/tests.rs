use super::*;

use std::io::Write;

const HELLO_WORLD: &[u8] = b"Hello, world!";

#[test]
fn splitting_works() {
    let mut hello_world = LoafSlice::new(HELLO_WORLD.len());
    hello_world.as_slice_mut().write_all(HELLO_WORLD).unwrap();
    let [hello, world] = hello_world.split_at(6);
    assert_eq!(hello.as_slice(), b"Hello,");
    assert_eq!(world.as_slice(), b" world!");
}

#[test]
fn splitting_out_of_bounds_ok() {
    let mut hello_world = LoafSlice::new(HELLO_WORLD.len());
    hello_world.as_slice_mut().write_all(HELLO_WORLD).unwrap();
    let nothing = hello_world.split_after(300);
    assert_eq!(hello_world.as_slice(), HELLO_WORLD);
    assert_eq!(nothing.as_slice(), b"");
}

#[test]
fn joining() {
    let [a, b] = LoafSlice::new(10).split_at(5);
    let _ab = a.try_joined(b).unwrap();
}
