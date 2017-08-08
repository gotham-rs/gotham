use state::{State, request_id};

use hyper::{Headers, Uri, HttpVersion, Method};

/// A trait for accessing data that is known to be stored in `State`.
///
/// This is especially applicable to data which was been extracted by the `Router` such as `Request`
/// path and query strings.
///
/// # Panics
/// All functions MUST panic if the value is a None when retrieved from `State`.
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

macro_rules! from_state {
    ($($t:ident),*) => { $(
        impl FromState<$t> for $t {
            fn take_from(s: &mut State) -> Self {
                s.take::<$t>()
                 .unwrap_or_else(|| {
                     panic!("[{}] [take] {} is not stored in State",
                            request_id(s), "$t")
                 })
            }

            fn borrow_from(s: &State) -> &$t {
                s.borrow::<$t>()
                 .unwrap_or_else(|| {
                     panic!("[{}] [borrow] {} is not stored in State",
                            request_id(s),
                            "$t")
                 })
            }

            fn borrow_mut_from(s: &mut State) -> &mut $t {
                let req_id = String::from(request_id(s));
                s.borrow_mut::<$t>()
                 .unwrap_or_else(|| {
                     panic!("[{}] [borrow_mut] {} is not stored in State",
                            req_id,
                            "$t")
                 })
            }
        }
    )+}
}

from_state!(Headers, Uri, HttpVersion, Method);
