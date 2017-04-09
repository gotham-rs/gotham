use middleware::Middleware;
use handler::{Handler, HandlerFuture};
use state::State;
use hyper::server::{Request, Response};

pub struct Pipeline {
    f: Box<Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync>,
}

impl Handler for Pipeline {
    fn handle(&self, req: Request) -> Box<HandlerFuture> {
        let mut s = State::new();
        (self.f)(&mut s, req)
    }
}

impl Pipeline {
    pub fn new() -> PipeEnd {
        PipeEnd { _nothing: () }
    }
}

pub trait PipelineBuilder: Sized {
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static;

    fn build<H>(self, handler: H) -> Pipeline
        where H: Handler + 'static
    {
        self.build_recurse(move |state: &mut State, req: Request| handler.handle(req))
    }

    fn add<M>(self, m: M) -> PipeSegment<M, Self>
        where M: Middleware + Send + Sync
    {
        PipeSegment {
            middleware: m,
            tail: self,
        }
    }
}

pub struct PipeSegment<M, Tail>
    where M: Middleware + Send + Sync,
          Tail: PipelineBuilder
{
    middleware: M,
    tail: Tail,
}

pub struct PipeEnd {
    _nothing: (),
}

impl<M, Tail> PipelineBuilder for PipeSegment<M, Tail>
    where M: Middleware + Send + Sync + 'static,
          Tail: PipelineBuilder
{
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static
    {
        let middleware = self.middleware;
        self.tail.build_recurse(move |state: &mut State, req: Request| {
                                    middleware.call(state, req, &f)
                                })
    }
}

impl PipelineBuilder for PipeEnd {
    fn build_recurse<F>(self, f: F) -> Pipeline
        where F: Fn(&mut State, Request) -> Box<HandlerFuture> + Send + Sync + 'static
    {
        Pipeline { f: Box::new(f) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::TestServer;
    use handler::HandlerService;
    use state::StateData;
    use hyper::server::Response;
    use hyper::StatusCode;

    fn handler(/*_state: &mut State,*/
               _req: Request)
               -> Response {
        Response::new().with_status(StatusCode::Ok).with_body("21")
    }

    #[derive(Clone)]
    struct Number {
        value: i32,
    }

    impl Middleware for Number {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
                  Self: Sized
        {
            state.put(self.clone());
            chain(state, req)
        }
    }

    impl StateData for Number {}

    struct Addition {
        value: i32,
    }

    impl Middleware for Addition {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value += self.value;
            chain(state, req)
        }
    }

    struct Multiplication {
        value: i32,
    }

    impl Middleware for Multiplication {
        fn call<Chain>(&self, state: &mut State, req: Request, chain: Chain) -> Box<HandlerFuture>
            where Chain: FnOnce(&mut State, Request) -> Box<HandlerFuture>,
                  Self: Sized
        {
            state.borrow_mut::<Number>().unwrap().value *= self.value;
            chain(state, req)
        }
    }

    #[test]
    fn pipeline_ordering_test() {
        let new_service = || {
            let pipeline = Pipeline::new()
                .add(Number { value: 0 }) // 0
                .add(Addition { value: 1 }) // 1
                .add(Multiplication { value: 2 }) // 2
                .add(Addition { value: 1 }) // 3
                .add(Multiplication { value: 2 }) // 6
                .add(Addition { value: 1 }) // 7
                .add(Multiplication { value: 3 }) // 21
                .build(handler);
            Ok(HandlerService::new(pipeline))
        };

        let uri = "http://localhost/".parse().unwrap();

        let mut test_server = TestServer::new(new_service).unwrap();
        let response = test_server.client("127.0.0.1:0".parse().unwrap()).unwrap().get(uri);
        let response = test_server.run_request(response).unwrap();

        let buf = test_server.read_body(response).unwrap();
        assert_eq!(buf.as_slice(), "21".as_bytes());
    }
}
