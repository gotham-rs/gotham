use state::State;

/// A trait for accessing data that is known to be stored in `State`.
///
/// This is especially applicable to data which was been extracted by the `Router` such as `Request`
/// path and query strings.
///
/// # Panics
/// All functions will panic if the value is a None when retrieved from `State`.
pub trait FromState<T> {
    /// Moves out of `State` and returns ownership
    ///
    /// # Panics
    /// When Self was not stored in State
    fn take_from(&mut State) -> T;

    /// Borrows from `State` storage
    ///
    /// # Panics
    /// When Self was not stored in State
    fn borrow_from(&State) -> &T;


    /// Mutably borrows from `State` storage
    ///
    /// # Panics
    /// When Self was not stored in State
    fn borrow_mut_from(&mut State) -> &mut T;
}
