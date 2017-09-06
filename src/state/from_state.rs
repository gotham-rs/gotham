use state::{State, StateData};

/// A trait for accessing data that is stored in `State`.
pub trait FromState: StateData + Sized {
    /// Tries to borrow a value from the `State` storage.
    fn try_borrow_from(&State) -> Option<&Self>;

    /// Borrows a value from the `State` storage.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    fn borrow_from(&State) -> &Self;

    /// Tries to mutably borrow a value from the `State` storage.
    fn try_borrow_mut_from(&mut State) -> Option<&mut Self>;

    /// Mutably borrows a value from the `State` storage.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    fn borrow_mut_from(&mut State) -> &mut Self;

    /// Tries to move a value out of the `State` storage and return ownership.
    fn try_take_from(&mut State) -> Option<Self>;

    /// Moves a value out of the `State` storage and returns ownership.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    fn take_from(&mut State) -> Self;
}

impl<T> FromState for T
where
    T: StateData,
{
    fn try_borrow_from(state: &State) -> Option<&Self> {
        state.try_borrow()
    }

    fn borrow_from(state: &State) -> &Self {
        state.borrow()
    }

    fn try_borrow_mut_from(state: &mut State) -> Option<&mut Self> {
        state.try_borrow_mut()
    }

    fn borrow_mut_from(state: &mut State) -> &mut Self {
        state.borrow_mut()
    }

    fn try_take_from(state: &mut State) -> Option<Self> {
        state.try_take()
    }

    fn take_from(state: &mut State) -> Self {
        state.take()
    }
}
