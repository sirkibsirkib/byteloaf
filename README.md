# Byte Loaf 🍞

## What
This library lets one treat a heap-allocated byte buffer as a loaf of bread; its contents can be arbitrarily partitioned into slices.

Concretely, a single-part loaf can be created, and treated as a mutable byte buffer as usual.
```rust
let x = LoafPart::new(5);

x.as_slice().write_all(b"hello").unwrap();
assert_eq!(x.as_slice(), b"hello");

x.as_slice_mut()[3] = b'Q';
assert_eq!(x.as_slice(), b"helQo");
```

Ownership of the loaf's bytes can be further sub-divided by splitting existing parts.
```rust
let y = x.split_at(3);
assert_eq!(x.as_slice(), b"hel"  );
assert_eq!(y.as_slice(),    b"lo");
```

For parts owning contiguous bytes, their sub-division of ownership can be re-drawn, or joined into one part.
```rust
x.with_try_resplit_at(y, 0..4).unwrap();
assert_eq!(x.as_slice(), b"hell" );
assert_eq!(y.as_slice(),     b"o");
let z = x.with_try_join(y).unwrap();
assert_eq!(z.as_slice(), b"hello");
```

## Why
This library was created as a utility for storing independently-owned byte slices, while minimizing the number of heap allocations.
For example, this is useful for hanging onto the contents of sent UDP datagrams until their receipt is acknowledged later.