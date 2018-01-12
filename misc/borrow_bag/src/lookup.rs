use handle::{Skip, Take};

/// Allows borrowing a value of type `T` from the implementing type. This can be used to constrain
/// a `Handle` argument to ensure it can be used with the corresponding `BorrowBag`.
///
/// # Examples
///
/// ```rust
/// # use borrow_bag::*;
/// #
/// fn borrow_from<V, T, N>(bag: &BorrowBag<V>, handle: Handle<T, N>) -> &T
///     where V: Lookup<T, N>
/// {
///     bag.borrow(handle)
/// }
/// #
/// # fn main() {
/// #   let bag = BorrowBag::new();
/// #   let (bag, handle) = bag.add(1u8);
/// #
/// #   assert_eq!(1u8, *borrow_from(&bag, handle));
/// # }
/// ```
pub trait Lookup<T, N> {
    /// Borrows the value of type `T`. Internal API and not for public use.
    #[doc(hidden)]
    fn get_from(&self) -> &T;
}

#[doc(hidden)]
impl<T, U, V, N> Lookup<T, (Skip, N)> for (U, V)
where
    V: Lookup<T, N>,
{
    fn get_from(&self) -> &T {
        self.1.get_from()
    }
}

#[doc(hidden)]
impl<T, V> Lookup<T, Take> for (T, V) {
    fn get_from(&self) -> &T {
        &self.0
    }
}
