//! Defines types for passing request state through `Middleware` and `Handler` implementations

pub mod request_id;

use std::collections::HashMap;
use std::any::{Any, TypeId};

pub use self::request_id::request_id;
pub use self::request_id::set_request_id;

/// Provides storage for request state, and stores one item of each type. The types used for
/// storage must implement the `gotham::state::StateData` trait to allow its storage.
pub struct State {
    data: HashMap<TypeId, Box<Any + Send>>,
}

/// A marker trait for types that can be stored in `State`.
pub trait StateData: Any + Send {}

impl State {
    /// Creates a new, empty `State`
    pub fn new() -> State {
        State { data: HashMap::new() }
    }

    /// Puts a value into the `State` storage. One value of each type is retained. Successive calls
    /// to `put` will overwrite the existing object of the same type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::state::{State, StateData};
    /// #
    /// # struct MyStruct {
    /// #     value: i32
    /// # }
    /// #
    /// # impl StateData for MyStruct {}
    /// #
    /// # struct AnotherStruct {
    /// #     value: &'static str
    /// # }
    /// #
    /// # impl StateData for AnotherStruct {}
    /// #
    /// # fn main() {
    /// # let mut state = State::new();
    /// #
    /// state.put(MyStruct { value: 1 });
    /// assert_eq!(state.borrow::<MyStruct>().unwrap().value, 1);
    ///
    /// state.put(AnotherStruct { value: "a string" });
    /// state.put(MyStruct { value: 100 });
    ///
    /// assert_eq!(state.borrow::<AnotherStruct>().unwrap().value, "a string");
    /// assert_eq!(state.borrow::<MyStruct>().unwrap().value, 100);
    /// # }
    /// ```
    pub fn put<T>(&mut self, t: T)
        where T: StateData
    {
        let type_id = TypeId::of::<T>();
        trace!(" inserting record to state for type_id `{:?}`", type_id);
        self.data.insert(type_id, Box::new(t));
    }

    /// Determines if the current value exists in `State` storage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::state::{State, StateData};
    /// #
    /// # struct MyStruct {
    /// #     value: i32
    /// # }
    /// #
    /// # impl StateData for MyStruct {}
    /// #
    /// # struct AnotherStruct {
    /// # }
    /// #
    /// # impl StateData for AnotherStruct {}
    /// #
    /// # fn main() {
    /// # let mut state = State::new();
    /// #
    /// state.put(MyStruct { value: 1 });
    /// assert!(state.has::<MyStruct>());
    /// assert_eq!(state.borrow::<MyStruct>().unwrap().value, 1);
    ///
    /// assert!(!state.has::<AnotherStruct>());
    /// # }
    /// ```
    pub fn has<T>(&self) -> bool
        where T: StateData
    {
        let type_id = TypeId::of::<T>();
        self.data.get(&type_id).is_some()
    }

    /// Borrows a value from the `State` storage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::state::{State, StateData};
    /// #
    /// # struct MyStruct {
    /// #     value: i32
    /// # }
    /// #
    /// # impl StateData for MyStruct {}
    /// #
    /// # struct AnotherStruct {
    /// # }
    /// #
    /// # impl StateData for AnotherStruct {}
    /// #
    /// # fn main() {
    /// # let mut state = State::new();
    /// #
    /// state.put(MyStruct { value: 1 });
    /// assert!(state.borrow::<MyStruct>().is_some());
    /// assert_eq!(state.borrow::<MyStruct>().unwrap().value, 1);
    ///
    /// assert!(state.borrow::<AnotherStruct>().is_none());
    /// # }
    /// ```
    pub fn borrow<T>(&self) -> Option<&T>
        where T: StateData
    {
        let type_id = TypeId::of::<T>();
        trace!(" borrowing state data for type_id `{:?}`", type_id);
        self.data.get(&type_id).and_then(|b| b.downcast_ref::<T>())
    }

    /// Mutably borrows a value from the `State` storage.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::state::{State, StateData};
    /// #
    /// # struct MyStruct {
    /// #     value: i32
    /// # }
    /// #
    /// # impl StateData for MyStruct {}
    /// #
    /// # struct AnotherStruct {
    /// # }
    /// #
    /// # impl StateData for AnotherStruct {}
    /// #
    /// # fn main() {
    /// # let mut state = State::new();
    /// #
    /// state.put(MyStruct { value: 100 });
    ///
    /// {
    ///     let a = state.borrow_mut::<MyStruct>().unwrap();
    ///     a.value += 10;
    /// }
    ///
    /// assert_eq!(state.borrow::<MyStruct>().unwrap().value, 110);
    ///
    /// assert!(state.borrow_mut::<AnotherStruct>().is_none());
    /// # }
    pub fn borrow_mut<T>(&mut self) -> Option<&mut T>
        where T: StateData
    {
        let type_id = TypeId::of::<T>();
        trace!(" mutably borrowing state data for type_id `{:?}`", type_id);
        self.data
            .get_mut(&type_id)
            .and_then(|b| b.downcast_mut::<T>())
    }

    /// Moves a value out of the `State` storage, and returns ownership.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate gotham;
    /// #
    /// # use gotham::state::{State, StateData};
    /// #
    /// # struct MyStruct {
    /// #     value: i32
    /// # }
    /// #
    /// # impl StateData for MyStruct {}
    /// #
    /// # struct AnotherStruct {
    /// # }
    /// #
    /// # impl StateData for AnotherStruct {}
    /// #
    /// # fn main() {
    /// # let mut state = State::new();
    /// #
    /// state.put(MyStruct { value: 110 });
    ///
    /// assert_eq!(state.take::<MyStruct>().unwrap().value, 110);
    ///
    /// assert!(state.take::<MyStruct>().is_none());
    /// assert!(state.borrow_mut::<MyStruct>().is_none());
    /// assert!(state.borrow::<MyStruct>().is_none());
    ///
    /// assert!(state.take::<AnotherStruct>().is_none());
    /// # }
    pub fn take<T>(&mut self) -> Option<T>
        where T: StateData
    {
        let type_id = TypeId::of::<T>();
        trace!(" taking ownership from state data for type_id `{:?}`",
               type_id);
        self.data
            .remove(&type_id)
            .and_then(|b| b.downcast::<T>().ok())
            .map(|b| *b)
    }
}
