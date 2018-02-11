//! Handlers are not a focal point of this example.
//!
//! We've used a macro here for brevity but this is NOT how you would implement a handler in
//! a real world application.

use gotham::http::response::create_response;
use gotham::state::State;
use hyper::{Response, StatusCode};
use mime;

macro_rules! generic_handler {
    ($($t:ident),*) => { $(
        pub fn $t(state: State) -> (State, Response) {
            let res = create_response(
                &state,
                StatusCode::Ok,
                Some((String::from(stringify!($t)).into_bytes(), mime::TEXT_PLAIN)),
            );

            (state, res)
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
    generic_handler!(start, complete);

    pub mod address {
        use super::*;
        generic_handler!(create, update, delete);
    }

    pub mod payment_details {
        use super::*;
        generic_handler!(create, update);
    }
}

pub mod api {
    use super::*;
    pub mod products {
        use super::*;
        generic_handler!(index);
    }
}
