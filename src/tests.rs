use super::*;

const HELLO_WORLD: &[u8] = b"HELLO_WORLD";

#[test]
fn new() {
    LoafPart::new(HELLO_WORLD.len());
}
#[test]
fn new_len_ok() {
    for len in 10..14 {
        assert_eq!(len, LoafPart::new(len).len());
    }
}

#[test]
fn new_from_slice() {
    LoafPart::new_from_slice(HELLO_WORLD);
}

#[test]
fn splitting_works() {
    let hello_world = LoafPart::new_from_slice(HELLO_WORLD);
    println!("{:?} {}", hello_world.get_ptr_range(), hello_world.len());
    let [hello, world] = hello_world.with_try_split_at(5).unwrap();
    println!("{:?} {}", hello.get_ptr_range(), hello.len());
    println!("{:?} {}", world.get_ptr_range(), world.len());
    assert_eq!(hello.as_slice(), b"HELLO");
    assert_eq!(world.as_slice(), b"_WORLD");
}

#[test]
fn resplitting() {
    let [mut a, mut b] = LoafPart::new(10).with_try_split_at(5).unwrap();

    LoafPart::try_resplit_at(&mut a, &mut b, 3).unwrap();
    assert_eq!([a.len(), b.len()], [3, 7]);

    LoafPart::try_resplit_at(&mut a, &mut b, 8).unwrap();
    assert_eq!([a.len(), b.len()], [8, 2]);
}

#[test]
fn joining() {
    let [a, b] = LoafPart::new(10).with_try_split_at(5).unwrap();
    a.with_try_join(b).unwrap();
}

#[test]
fn new_range() {
    let llo_w = LoafPart::new_from_slice(HELLO_WORLD)
        .with_try_set_relative_range(2..7)
        .unwrap();
    assert_eq!(llo_w.as_slice(), b"LLO_W");
}
