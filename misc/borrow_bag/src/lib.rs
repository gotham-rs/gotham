//! A type-safe, heterogeneous collection with zero-cost add and borrow.
//!
//! `BorrowBag` allows the storage of any value, and returns a `Handle` which can be used to borrow
//! the value back later. As the `BorrowBag` is add-only, `Handle` values remain valid for the
//! lifetime of the `BorrowBag`.

#![doc(html_root_url = "https://docs.rs/borrow-bag/0.4.0")] // Update when changed in Cargo.toml
#![warn(missing_docs, deprecated)]
// Stricter requirements once we get to pull request stage, all warnings must be resolved.
#![cfg_attr(feature = "ci", deny(warnings))]
#![doc(test(attr(deny(warnings))))]
// TODO: Remove this when it's a hard error by default (error E0446).
// See Rust issue #34537 <https://github.com/rust-lang/rust/issues/34537>
#![deny(private_in_public)]

mod append;
mod handle;
mod lookup;

pub use append::Append;
pub use lookup::Lookup;
pub use handle::Handle;

/// Creates a new, empty `BorrowBag`.
#[deprecated(since = "0.4.0", note = "use `BorrowBag::new()`")]
pub fn new_borrow_bag() -> BorrowBag<()> {
    BorrowBag::new()
}

/// `BorrowBag` allows the storage of any value using `add(T)`, and returns a `Handle` which can be
/// used to borrow the value back later. As the `BorrowBag` is add-only, `Handle` values remain
/// valid for the lifetime of the `BorrowBag`.
///
/// After being added, the `Handle` can be passed to `borrow(Handle)`, which will return a
/// reference to the value.
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
    pub fn borrow<T, N>(&self, handle: Handle<T, N>) -> &T
    where
        V: Lookup<T, N>,
    {
        drop(handle); // Otherwise it's unused.
        Lookup::<T, N>::get_from(&self.v)
    }
}
