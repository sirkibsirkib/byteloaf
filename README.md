# Byteloaf

## What
This library provides types for safely managing access to a heap-allocated 'loaf' of contiguous bytes, accessible via slicing.
```rust
let a = LoafSlice::new(32);
assert_eq!(a.as_slice().len(), 32);
```

Slices can be split at a relative index, safely subdividing access to the underlying loaf.
```rust
let a = LoafSlice::new(32);
let [a, b] = a.slice_at(30);
assert_eq!(a.as_slice().len(), 30);
assert_eq!(b.as_slice().len(),  2);
```

Slices share ownership of their loaves, so they can be moved around hassle-free. The loaf is freed as the last of its slices is dropped.


## Why
This library was created for use as part of another project, which needed an input buffer for reading TCP sequents which encode large, serialized data structures. Rather than reading into a `Vec<u8>`, it's handy to subdivide the input buffer, such that partially-received message data can be left in-place to be constructed later, while other messages arrive. 