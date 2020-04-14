# DMA Buffer Type

Assuming we have a `Transfer` struct abstracting a DMA transfer:

```rust
struct Transfer<B, P> {
    buffer: B,   // the buffer DMA reads from/writes to
    payload: P,  // owned DMA channel + peripheral
}
```

**Question:** What buffer types `B` are safe to use?


## Specific Types

This section lists a few specific types that are commonly used with DMA and
should be supported by whatever we come up with.

Let `T` be any type.

Let `S` be any type for which any byte pattern is valid. That is, any type
that fulfills [`zerocopy::FromBytes`].

```rust
S: zerocopy::FromBytes
```

### Safe for DMA reads and writes

- `&'static mut S`
- `alloc::boxed::Box<S>`
- `alloc::vec::Vec<S>`
- `bbqueue::GrantW`
- `heapless::pool::Box`

### Safe for DMA reads only

Shared references:

- `&'static T`
- `alloc::rc::Rc<T>`
- `alloc::arc::Arc<T>`
- `bbqueue::GrantR`

Have invalid byte patterns:

- `alloc::string::String`


[`zerocopy::FromBytes`]: https://docs.rs/zerocopy/0.3.0/zerocopy/trait.FromBytes.html


## Requirements

### Requirement 1: `B` must be a pointer

That is, `B` must point to another location in memory where the actual
buffer is located.

That is, the actual buffer must not be part of the `Transfer` struct.
Otherwise is would be moved around the stack when the `Transfer` struct
is passed between functions. Its address would change without the DMA
knowing, and the DMA would read from/write to invalid memory locations.

See [examples/unsound-non-pointer.rs].

### Solutions

- Requiring `B` to fulfill the `StableDeref` bound enforces this requirement.

  ```rust
  B: Deref + StableDeref    // for DMA reads
  B: DerefMut + StableDeref // for DMA writes
  ```

- **[unsound]** Wrapping `B` in a `Pin` does *not* satisfy this requirement as
  `Pin` doesn't provide sufficient guarantees.

  See [examples/unsound-pin.rs].


### Requirement 2: `B::Target` must be stable

That is, dereferencing `B` must always yield the same memory location.
Otherwise it is not guaranteed that the buffer stays valid for the whole
duration of the DMA transfer.

As an example, consider this unstable buffer type:

```rust
struct UnstableBuffer {
    inner: Box<[u8; 16]>,
}

impl DerefMut for UnstableBuffer {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.inner = Box::new([0; 16]);
        &mut self.inner
    }
}
```

Every time we request a mutable reference to this buffer, it frees the
buffer it currently holds and returns a new one. If DMA is already running
on the old buffer, it would then access freed memory, which is unsafe.

This requirement can be lifted if we can ensure that no call to `B::deref_mut`
happens after a DMA transfer was started on `B`. This seems overly restrictive,
since it makes it impossible to, e.g., to provide the user with a way to write
to the part of the buffer the DMA doesn't currently access.

### Solutions

- Requiring `B` to fulfill the `StableDeref` bound enforces this requirement.


### Requirement 3: `B::Target` must stay valid if `B` is forgotten

Calling [`mem::forget`] is safe. This means it is possible, in safe Rust, to
lose our handle to the `Transfer` struct without having its destructor run.
This means we cannot rely on the `Transfer` destructor to guarantee memory
safety (by stopping or waitings for the DMA transfer). This means, `B::Target`
must remain a valid (i.e. not freed) buffer, as long as `B` is not dropped.

This requirement does not hold for, e.g., references to stack buffers.
See [examples/unsound-non-static.rs].

### Solutions

- Adding a `'static` bound, together with the bounds from Requirement 1,
  enforces this requirement. This way, we allow only:

  1. Pointers to static memory (`&'static T`, `MyBuffer<'static, T>`):

     Buffers in static memory will never be freed, so the DMA can safely
     continue accessing them in the background.

  2. Owned pointer types (`Box<T>`, `Vec<T>`, `Rc<T>`, `String`, ...):

     Since those pointers own the memory they point to, that memory won't
     be freed until the pointer is dropped. Since `mem::forget` explicitly
     does not drop whatever is passed to it, the memory will not be freed
     either.

  3. Shared pointer types (`Rc<T>`, `Arc<T>`):

     The memory those pointers reference won't be dropped as long as there
     is still a (shared) reference to it (i.e. reference count > 0). Again,
     since `mem::forget` prevents dropping, a shared pointer's reference
     will never be given back, so the referenced buffer won't be freed.

- The `'static` bound can be dropped if the user of the `Transfer` promises
  to never call `mem::forget` on it or leak it in any other way. This cannot
  be expressed in the type system, unfortunately. But we can provide an `unsafe` constructor method for `Transfer` and document this requirement there.

[`mem::forget`]: https://doc.rust-lang.org/core/mem/fn.forget.html
[examples/unsound-non-pointer.rs]: examples/unsound-non-pointer.rs
[examples/unsound-non-static.rs]: examples/unsound-non-static.rs
[examples/unsound-pin.rs]: examples/unsound-pin.rs


## Open Questions

- What are other specific buffer types we want to support?
  - Common `heapless` types?
    - Probably not generally possible, because they can be allocated on the
      stack
    - `heapless::pool::Box` is an exception

- Are the above requirements on `B` enough to ensure safe DMA?
  - Currently we have:

    ```rust
    B: Deref + StableDeref + 'static    // for DMA reads
    B: DerefMut + StableDeref + 'static // for DMA writes
    ```
  - Can we find counter examples that fulfill these bounds and still lead
    to unsafety?

- What requirements do we need for `B::Target` itself?
  - Our current `As(Mut)Slice` may be not good enough as that's also not
    guaranteed to be stable.
    - On the other hand `AsSlice` may be fine since it shouldn't be able to
      drop our DMA buffer, since it only gets an immutable reference?
    - And maybe we don't need `AsMutSlice`?
  - Do we need to restrict the type of elements the buffer can have, to
    ensure a DMA write doesn't create values with invalid bit-patterns?
    - If so, should we just restrict it elements to integer types?
    - Or do we want some trait that codifies the "can be safely cast from
      byte" property?
        - Prior art: [`zerocopy.FromBytes`](https://docs.rs/zerocopy/0.3.0/zerocopy/trait.FromBytes.html)
  - Do we want to discuss alignment here?
    - Probably not, can be done separately.
    - We should just make sure our final recommendation doesn't prevent
      common approaches to specifying alignment requirements.

- What `Target` should `B` have ?
  - The final type for the DMA must be a slice of some sort. But with a strict specification (i.e. `Target = [T]`) we lose some flexibility, one common use case is to be able to pass in a wrapper type around `MaybeUninit`s or even a `Box<[u8; N]>`.
  - We could also accept wrapper types where `B::Target: Deref<Target = [T]>`, but we need some way to ensure that this second `deref` will not invalidate the `StableDeref` guarantees of `B`, do we need another trait here to express this requirement ? Example that works both with slices or "wrapper" types around slices:
    ```rust
    use core::ops::Deref;
    use core::convert::AsRef;

    fn print_it<B>(buffer: B)
    where
      B: Deref,
      // AsRef is used as an example here, but it doesn't guarantee our requirements
      B::Target: AsRef<[u8]>,
    {
      println!("{:?}", &*buffer.as_ref());
    }

    fn main() {
      let buffer = Box::new([1u8; 4]);
      let buffer2 = [0u8; 4];
      print_it(buffer);
      print_it(&buffer2[..])
    }
    ```
