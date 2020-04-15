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

Note: Most of the below types reference `[u8]` buffers. Aside from `u8`, we'd
like to support at least the larger word sizes `u16` and `u32`.

### Safe for DMA reads and writes

- `&'static mut [u8]`
- `alloc::boxed::Box<[u8]>`
- `alloc::vec::Vec<u8>`
- `bbqueue::GrantW<'static, N>`
- `heapless::pool::Box<[u8], _>`

### Safe for DMA reads only

Shared/read-only references:

- `&'static [u8]`
- `alloc::rc::Rc<[u8]>`
- `alloc::arc::Arc<[u8]>`
- `bbqueue::GrantR<'static, N>`

Have invalid byte patterns:

- `alloc::string::String`


## Requirements

### Requirement 1: `B` must be a pointer

That is, `B` must point to another location in memory where the actual
buffer is located.

That is, the actual buffer must not be part of the `Transfer` struct.
Otherwise is would be moved around the stack when the `Transfer` struct
is passed between functions. Its address would change without the DMA
knowing, and the DMA would read from/write to invalid memory locations.

See [examples/unsound-non-pointer.rs].

#### Solutions

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

#### Solutions

- Requiring `B` to fulfill the `StableDeref` bound enforces this requirement.


### Requirement 3: `B::Target` must stay valid if `B` is forgotten

Calling [`mem::forget`] is safe. This means it is possible, in safe Rust, to
lose our handle to the `Transfer` struct without having its destructor run.
This means we cannot rely on the `Transfer` destructor to guarantee memory
safety (by stopping or waitings for the DMA transfer). This means, `B::Target`
must remain a valid (i.e. not freed) buffer, as long as `B` is not dropped.

This requirement does not hold for, e.g., references to stack buffers.
See [examples/unsound-non-static.rs].

#### Solutions

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


### Requirement 4: `B::Target` must be valid for every possible byte pattern

When doing a DMA write into `B::Target`, we have no way to ensure at the
type system-level that the DMA writes only values that are valid according
to `B::Target`'s type. Since [producing an invalid value leads to UB][ub],
we can only allow target types for which all byte patterns are known to be
valid values.

This is only a necessary requirement for DMA writes. It might be sensible to
enforce it for DMA reads too, though, for the sake of symmetry and sanity.

#### Solutions

- Allow only the common DMA buffer types `[u8]`, `[u16]`, `[u32]`. These
  are known to be always valid, regardless of the underlying byte pattern.
  It's not clear if there is any practical need for supporting other types,
  especially because everything can be cast to a `[u8]` if necessary.

- Introduce a new marker trait for types that are valid for every byte pattern
  and bound `B::Target` on that. There is prior art in [`zerocopy::FromBytes`].
  This would introduce additional maintenance effort, since we probably don't
  want to depend on `zerocopy` directly, so we'd have to implement that
  ourselves.


[`mem::forget`]: https://doc.rust-lang.org/core/mem/fn.forget.html
[`zerocopy::FromBytes`]: https://docs.rs/zerocopy/0.3.0/zerocopy/trait.FromBytes.html
[examples/unsound-non-pointer.rs]: examples/unsound-non-pointer.rs
[examples/unsound-non-static.rs]: examples/unsound-non-static.rs
[examples/unsound-pin.rs]: examples/unsound-pin.rs
[ub]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html


## Open Questions

- Are the above requirements on `B` enough to ensure safe DMA?
  - Currently we have:

    ```rust
    // for DMA reads:
    B: Deref<Target = [Word]> + StableDeref + 'static,

    // for DMA writes:
    B: DerefMut<Target = [Word]> + StableDeref + 'static
    ```

    ... with `Word` implemented for `u8`, `u16`, `u32`.

  - Can we find counter examples that fulfill these bounds and still lead

- The above trait bounds are too restrictive as the allow only `[Word]` buffers
  - We also want to support:
    - `[Word; N]`
    - `Wrapper([Word; N])`
    - `MaybeUninit([Word; N])` (DMA writes only)
    - ... (what else?)
  - For DMA reads requiring `B::Target: AsSlice<Element = Word>`  should be fine
  - For DMA writes:
    - We can *not* use `AsSlice` as that would enable producing invalid
      values for some buffer types (see [examples/unsound-asref.rs])
    - We can *not* use `AsMutSlice` as that would make it possible to break
      Requirement 2 again
    - We could use some `unsafe` marker trait that makes implementors promise
      `as_mut_slice` always returns the same slice ("`StableAsSlice`")
    - However, a solution based on `AsMutSlice` makes using `MaybeUninit`
      impossible, since it is UB to get a reference to an uninitialized value

- Do we want to discuss alignment here?
  - Probably not, can be done separately.
  - We should just make sure our final recommendation doesn't prevent
    common approaches to specifying alignment requirements.


[examples/unsound-asref.rs]: examples/unsound-asref.rs
