//! A type-safe, heterogeneous collection with zero-cost add and borrow.
//!
//! `BorrowBag` allows the storage of any value, and returns a `Handle` which can be used to borrow
//! the value back later. As the `BorrowBag` is add-only, `Handle` values remain valid for the
//! lifetime of the `BorrowBag`.

#![doc(html_root_url = "https://docs.rs/borrow-bag/1.1.1")] // Update when changed in Cargo.toml
#![allow(clippy::should_implement_trait)]
#![warn(missing_docs)]
#![forbid(elided_lifetimes_in_paths, unsafe_code)]
#![doc(test(attr(deny(warnings))))]

mod append;
mod handle;
mod lookup;

pub use append::Append;
pub use handle::Handle;
pub use lookup::Lookup;

/// `BorrowBag` allows the storage of any value using `add(T)`, and returns a `Handle` which can be
/// used to borrow the value back later. As the `BorrowBag` is add-only, `Handle` values remain
/// valid for the lifetime of the `BorrowBag`.
///
/// After being added, the `Handle` can be passed to `borrow(Handle)`, which will return a
/// reference to the value.
///
/// ## Example
///
/// ```rust
/// use borrow_bag::BorrowBag;
///
/// #[derive(PartialEq, Debug)]
/// struct X;
///
/// #[derive(PartialEq, Debug)]
/// struct Y;
///
/// #[derive(PartialEq, Debug)]
/// struct Z;
///
/// let bag = BorrowBag::new();
/// let (bag, x_handle) = bag.add(X);
/// let (bag, y_handle) = bag.add(Y);
/// let (bag, z_handle) = bag.add(Z);
///
/// let x: &X = bag.borrow(x_handle);
/// assert_eq!(x, &X);
/// let y: &Y = bag.borrow(y_handle);
/// assert_eq!(y, &Y);
/// let z: &Z = bag.borrow(z_handle);
/// assert_eq!(z, &Z);
///
/// // Can borrow multiple times using the same handle
/// let x: &X = bag.borrow(x_handle);
/// assert_eq!(x, &X);
/// ```

#[derive(Default)]
pub struct BorrowBag<V> {
    v: V,
}

impl BorrowBag<()> {
    /// Creates a new, empty `BorrowBag`.
    pub fn new() -> Self {
        BorrowBag { v: () }
    }
}

impl<V> BorrowBag<V> {
    /// Adds a value to the bag, and returns a tuple containing:
    ///
    /// 1. The new bag containing the added element; and
    /// 2. A `Handle` which can be used to retrieve the added element.
    ///
    /// The trait bound is used to constrain and define the `BorrowBag` implementation, but is not
    /// intended to provide any restrictions on the value being added.
    ///
    /// ```rust
    /// # use borrow_bag::BorrowBag;
    /// #
    /// let bag = BorrowBag::new();
    /// // Consumes the empty `bag`, and produces a new `bag` containing the value. The `handle`
    /// // can be used to borrow the value back later.
    /// let (bag, handle) = bag.add(15u8);
    /// #
    /// # let _ = (bag, handle);
    /// ```
    // This isn't add like +..
    // Consider renaming this method?
    pub fn add<T>(self, t: T) -> (BorrowBag<V::Output>, Handle<T, V::Navigator>)
    where
        V: Append<T>,
    {
        let (v, handle) = Append::append(self.v, t);
        (BorrowBag { v }, handle)
    }

    /// Borrows a value previously added to the bag.
    ///
    /// ```rust
    /// # use borrow_bag::BorrowBag;
    /// #
    /// # let bag = BorrowBag::new();
    /// # let (bag, handle) = bag.add(15u8);
    /// #
    /// let i: &u8 = bag.borrow(handle);
    /// assert_eq!(*i, 15);
    /// ```
    pub fn borrow<T, N>(&self, _handle: Handle<T, N>) -> &T
    where
        V: Lookup<T, N>,
    {
        Lookup::<T, N>::get_from(&self.v)
    }
}
