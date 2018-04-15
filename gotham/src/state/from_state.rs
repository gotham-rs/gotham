use state::{State, StateData};

/// A trait for accessing data that is stored in `State`.
///
/// This provides the easier `T::try_borrow_from(&state)` API (for example), as an alternative to
/// `state.try_borrow::<T>()`.
pub trait FromState: StateData + Sized {
    /// Tries to borrow a value from the `State` storage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// match MyStruct::try_borrow_from(&state) {
    ///     Some(&MyStruct { val }) => assert_eq!(val, "This is the value!"),
    ///     _ => panic!("expected `MyStruct` to be present"),
    /// }
    /// # });
    /// # }
    /// ```
    fn try_borrow_from(&State) -> Option<&Self>;

    /// Borrows a value from the `State` storage.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// let my_struct = MyStruct::borrow_from(&state);
    /// assert_eq!(my_struct.val, "This is the value!");
    /// # });
    /// # }
    /// ```
    fn borrow_from(&State) -> &Self;

    /// Tries to mutably borrow a value from the `State` storage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|mut state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// match MyStruct::try_borrow_mut_from(&mut state) {
    ///     Some(&mut MyStruct { ref mut val }) => *val = "This is the new value!",
    ///     _ => panic!("expected `MyStruct` to be present"),
    /// }
    /// #
    /// # assert_eq!(MyStruct::borrow_from(&state).val, "This is the new value!");
    /// # });
    /// # }
    /// ```
    fn try_borrow_mut_from(&mut State) -> Option<&mut Self>;

    /// Mutably borrows a value from the `State` storage.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|mut state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// # {
    /// let my_struct = MyStruct::borrow_mut_from(&mut state);
    /// my_struct.val = "This is the new value!";
    /// # }
    /// # assert_eq!(MyStruct::borrow_from(&state).val, "This is the new value!");
    /// # });
    /// # }
    /// ```
    fn borrow_mut_from(&mut State) -> &mut Self;

    /// Tries to move a value out of the `State` storage and return ownership.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|mut state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// match MyStruct::try_take_from(&mut state) {
    ///     Some(MyStruct { val }) => assert_eq!(val, "This is the value!"),
    ///     _ => panic!("expected `MyStruct` to be present"),
    /// }
    /// # });
    /// # }
    /// ```
    fn try_take_from(&mut State) -> Option<Self>;

    /// Moves a value out of the `State` storage and returns ownership.
    ///
    /// # Panics
    ///
    /// If `Self` is not present in `State`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # #[macro_use]
    /// # extern crate gotham_derive;
    /// #
    /// # use gotham::state::{FromState, State};
    /// #
    /// # fn main() {
    /// #[derive(StateData, Eq, PartialEq, Debug)]
    /// struct MyStruct {
    ///     val: &'static str,
    /// }
    ///
    /// # State::with_new(|mut state| {
    /// state.put(MyStruct { val: "This is the value!" });
    ///
    /// let my_struct = MyStruct::take_from(&mut state);
    /// assert_eq!(my_struct.val, "This is the value!");
    /// # });
    /// # }
    /// ```
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
