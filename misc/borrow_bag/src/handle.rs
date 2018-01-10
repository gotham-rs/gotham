use std::marker::PhantomData;

/// Navigator type describing a skipped element
pub struct Skip;

/// Navigator type describing the target element
pub struct Take;

/// A value which can be used with the `BorrowBag` to borrow the element which was added.
///
/// See [`BorrowBag`][BorrowBag] for usage examples.
///
/// [BorrowBag]: struct.BorrowBag.html
pub struct Handle<T, N> {
    phantom: PhantomData<(T, N)>,
}

/// Creates a new `Handle` of any given type.
pub fn new_handle<T, N>() -> Handle<T, N> {
    Handle { phantom: PhantomData }
}

impl<T, N> Clone for Handle<T, N> {
    fn clone(&self) -> Handle<T, N> {
        new_handle()
    }
}

// Derived `Copy` doesn't work here.
impl<T, N> Copy for Handle<T, N> {}
