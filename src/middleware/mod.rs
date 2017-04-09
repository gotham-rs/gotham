use handler::HandlerFuture;
use state::State;
use hyper::server::Request;

mod pipeline;

pub trait Middleware {
    fn call<Chain>(&self, &mut State, Request, Chain) -> Box<HandlerFuture>
        where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
              Self: Sized;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopMiddleware {}

    impl Middleware for NoopMiddleware {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>
        {
            chain(state, req)
        }
    }
}
