//! Handlers are not a focal point of this example.
//!
//! We've used a macro here for brevity but this is NOT how you would implement a handler in
//! a real world application.

use gotham::handler::IntoResponse;
use gotham::state::State;

macro_rules! generic_handler {
    ($($t:ident),*) => { $(
        pub fn $t(state: State) -> (State, impl IntoResponse) {
            (state, stringify!($t))
        }
    )+ }}

generic_handler!(index);

pub mod products {
    use super::*;
    generic_handler!(index);
}

pub mod bag {
    use super::*;
    generic_handler!(index);
}

pub mod checkout {
    use super::*;

    pub mod address {
        use super::*;
        generic_handler!(create, update, delete);
    }
}
