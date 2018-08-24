//! Defines a builder API for constructing a `Router`.

mod associated;
mod draw;
mod modify;
mod single;

use std::marker::PhantomData;
use std::panic::RefUnwindSafe;

use hyper::{Body, StatusCode};

use extractor::{NoopPathExtractor, NoopQueryStringExtractor, PathExtractor, QueryStringExtractor};
use pipeline::chain::PipelineHandleChain;
use pipeline::set::{finalize_pipeline_set, new_pipeline_set, PipelineSet};
use router::response::extender::ResponseExtender;
use router::response::finalizer::ResponseFinalizerBuilder;
use router::route::dispatch::DispatcherImpl;
use router::route::matcher::{AnyRouteMatcher, RouteMatcher};
use router::route::{Delegation, Extractors, RouteImpl};
use router::tree::node::Node;
use router::tree::Tree;
use router::Router;

pub use self::associated::{AssociatedRouteBuilder, AssociatedSingleRouteBuilder};
pub use self::draw::DrawRoutes;
pub use self::modify::{ExtendRouteMatcher, ReplacePathExtractor, ReplaceQueryStringExtractor};
pub use self::single::DefineSingleRoute;

/// Builds a `Router` using the provided closure. Routes are defined using the `RouterBuilder`
/// value passed to the closure, and the `Router` is constructed before returning.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// # #[macro_use]
/// # extern crate serde_derive;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::pipeline::new_pipeline;
/// # use gotham::pipeline::single::*;
/// # use gotham::middleware::session::{NewSessionMiddleware, SessionData};
/// # use gotham::test::TestServer;
/// #
/// # #[derive(Serialize, Deserialize, Default)]
/// # struct Session;
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #   assert!(state.has::<SessionData<Session>>());
/// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
/// # }
/// #
/// fn router() -> Router {
///     let (chain, pipelines) = single_pipeline(
///         new_pipeline()
///             .add(NewSessionMiddleware::default().with_session_type::<Session>())
///             .build()
///     );
///
///     build_router(chain, pipelines, |route| {
///         route.get("/request/path").to(my_handler);
///     })
/// }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(router()).unwrap();
/// #   let response = test_server.client()
/// #       .get("https://example.com/request/path")
/// #       .perform()
/// #       .unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
pub fn build_router<C, P, F>(pipeline_chain: C, pipelines: PipelineSet<P>, f: F) -> Router
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
    F: FnOnce(&mut RouterBuilder<C, P>),
{
    let mut tree = Tree::new();

    let response_finalizer = {
        let mut builder = RouterBuilder {
            node_builder: tree.borrow_root_mut(),
            pipeline_chain,
            pipelines,
            response_finalizer_builder: ResponseFinalizerBuilder::internal_new(),
        };

        f(&mut builder);

        builder.response_finalizer_builder.finalize()
    };

    Router::internal_new(tree, response_finalizer)
}

/// Builds a `Router` with **no** middleware using the provided closure. Routes are defined using
/// the `RouterBuilder` value passed to the closure, and the `Router` is constructed before
/// returning.
///
/// ```rust
/// # extern crate gotham;
/// # extern crate hyper;
/// #
/// # use hyper::{Body, Response, StatusCode};
/// # use gotham::state::State;
/// # use gotham::router::Router;
/// # use gotham::router::builder::*;
/// # use gotham::test::TestServer;
/// #
/// # fn my_handler(state: State) -> (State, Response<Body>) {
/// #   (state, Response::builder().status(StatusCode::ACCEPTED).body(Body::empty()).unwrap())
/// # }
/// #
/// fn router() -> Router {
///     build_simple_router(|route| {
///         route.get("/request/path").to(my_handler);
///     })
/// }
/// #
/// # fn main() {
/// #   let test_server = TestServer::new(router()).unwrap();
/// #   let response = test_server.client()
/// #       .get("https://example.com/request/path")
/// #       .perform()
/// #       .unwrap();
/// #   assert_eq!(response.status(), StatusCode::ACCEPTED);
/// # }
/// ```
pub fn build_simple_router<F>(f: F) -> Router
where
    F: FnOnce(&mut RouterBuilder<(), ()>),
{
    let pipelines = finalize_pipeline_set(new_pipeline_set());
    let pipeline_chain = ();

    build_router(pipeline_chain, pipelines, f)
}

/// The top-level builder which is created by `build_router` and passed to the provided closure.
/// See the `build_router` function and the `DrawRoutes` trait for usage.
pub struct RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut Node,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    response_finalizer_builder: ResponseFinalizerBuilder,
}

impl<'a, C, P> RouterBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    /// Adds a `ResponseExtender` to the `ResponseFinalizer` in the `Router`.
    ///
    /// ```rust
    /// # extern crate gotham;
    /// # extern crate hyper;
    /// #
    /// # use hyper::{Body, Response, StatusCode};
    /// # use hyper::header::WARNING;
    /// # use gotham::state::State;
    /// # use gotham::router::Router;
    /// # use gotham::router::response::extender::ResponseExtender;
    /// # use gotham::router::builder::*;
    /// # use gotham::test::TestServer;
    /// #
    /// # fn my_handler(state: State) -> (State, Response<Body>) {
    /// #   (state, Response::builder().status(StatusCode::INTERNAL_SERVER_ERROR).body(Body::empty()).unwrap())
    /// # }
    /// #
    /// struct MyExtender;
    ///
    /// impl ResponseExtender<Body> for MyExtender {
    ///     fn extend(&self, state: &mut State, response: &mut Response<Body>) {
    ///         // Extender implementation omitted.
    /// #       let _ = state;
    /// #       response.headers_mut().insert(WARNING, "299 example.com Deprecated".parse().unwrap());
    ///     }
    /// }
    ///
    /// fn router() -> Router {
    ///     build_simple_router(|route| {
    ///         route.add_response_extender(StatusCode::INTERNAL_SERVER_ERROR, MyExtender);
    /// #
    /// #       route.get("/").to(my_handler);
    ///     })
    /// }
    /// #
    /// # fn main() {
    /// #   let test_server = TestServer::new(router()).unwrap();
    /// #   let response = test_server.client()
    /// #       .get("https://example.com/")
    /// #       .perform()
    /// #       .unwrap();
    /// #   assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    /// #
    /// #   {
    /// #       let warning = response.headers().get(WARNING).unwrap();
    /// #       assert_eq!(warning, "299 example.com Deprecated");
    /// #   }
    /// # }
    /// ```
    pub fn add_response_extender<E>(&mut self, status_code: StatusCode, extender: E)
    where
        E: ResponseExtender<Body> + Send + Sync + 'static,
    {
        self.response_finalizer_builder
            .add(status_code, Box::new(extender))
    }
}

/// A scoped builder, which is created by `DrawRoutes::scope` and passed to the provided closure.
/// The `DrawRoutes` trait has documentation for using this type.
pub struct ScopeBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut Node,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

/// A delegated builder, which is created by `DrawRoutes::delegate` and returned. The `DrawRoutes`
/// trait has documentation for using this type.
pub struct DelegateRouteBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: Send + Sync + 'static,
{
    node_builder: &'a mut Node,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
}

type DelegatedRoute = RouteImpl<AnyRouteMatcher, NoopPathExtractor, NoopQueryStringExtractor>;

impl<'a, C, P> DelegateRouteBuilder<'a, C, P>
where
    C: PipelineHandleChain<P> + Copy + Send + Sync + 'static,
    P: RefUnwindSafe + Send + Sync + 'static,
{
    /// Directs the delegated route to the given `Router`.
    pub fn to_router(self, router: Router) {
        let dispatcher = DispatcherImpl::new(router, self.pipeline_chain, self.pipelines);
        let route: DelegatedRoute = DelegatedRoute::new(
            AnyRouteMatcher::new(),
            Box::new(dispatcher),
            Extractors::new(),
            Delegation::External,
        );

        self.node_builder.add_route(Box::new(route));
    }
}

/// Implements the traits required to define a single route, after determining which request paths
/// will be dispatched here. The `DefineSingleRoute` trait has documentation for using this type.
pub struct SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
{
    node_builder: &'a mut Node,
    matcher: M,
    pipeline_chain: C,
    pipelines: PipelineSet<P>,
    phantom: PhantomData<(PE, QSE)>,
}

// Trait impls live with the traits.
impl<'a, M, C, P, PE, QSE> SingleRouteBuilder<'a, M, C, P, PE, QSE>
where
    M: RouteMatcher + Send + Sync + 'static,
    C: PipelineHandleChain<P> + Send + Sync + 'static,
    P: Send + Sync + 'static,
    PE: PathExtractor<Body> + Send + Sync + 'static,
    QSE: QueryStringExtractor<Body> + Send + Sync + 'static,
{
    /// Coerces the type of the internal `PhantomData`, to replace an extractor by changing the
    /// type parameter without changing anything else.
    fn coerce<NPE, NQSE>(self) -> SingleRouteBuilder<'a, M, C, P, NPE, NQSE>
    where
        NPE: PathExtractor<Body> + Send + Sync + 'static,
        NQSE: QueryStringExtractor<Body> + Send + Sync + 'static,
    {
        SingleRouteBuilder {
            node_builder: self.node_builder,
            matcher: self.matcher,
            pipeline_chain: self.pipeline_chain,
            pipelines: self.pipelines,
            phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{Future, Stream};
    use hyper::service::Service;
    use hyper::{Body, Request, Response, StatusCode};

    use middleware::session::NewSessionMiddleware;
    use pipeline::new_pipeline;
    use router::response::extender::StaticResponseExtender;
    use service::GothamService;
    use state::{State, StateData};

    #[derive(Deserialize)]
    struct SalutationParams {
        name: String,
    }

    impl StateData for SalutationParams {}

    impl StaticResponseExtender for SalutationParams {
        type ResBody = Body;
        fn extend(_: &mut State, _: &mut Response<Body>) {}
    }

    #[derive(Deserialize)]
    struct AddParams {
        x: u64,
        y: u64,
    }

    impl StateData for AddParams {}

    impl StaticResponseExtender for AddParams {
        type ResBody = Body;
        fn extend(_: &mut State, _: &mut Response<Body>) {}
    }

    mod welcome {
        use super::*;
        pub fn index(state: State) -> (State, Response<Body>) {
            (
                state,
                Response::builder()
                    .status(StatusCode::OK)
                    .body(Body::empty())
                    .unwrap(),
            )
        }

        pub fn literal(state: State) -> (State, Response<Body>) {
            (
                state,
                Response::builder()
                    .status(StatusCode::CREATED)
                    .body(Body::empty())
                    .unwrap(),
            )
        }

        pub fn hello(mut state: State) -> (State, Response<Body>) {
            let params = state.take::<SalutationParams>();
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(format!("Hello, {}!", params.name).into())
                .unwrap();
            (state, response)
        }

        pub fn globbed(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body("Globbed".into())
                .unwrap();
            (state, response)
        }

        pub fn delegated(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body("Delegated".into())
                .unwrap();
            (state, response)
        }

        pub fn goodbye(mut state: State) -> (State, Response<Body>) {
            let params = state.take::<SalutationParams>();
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(format!("Goodbye, {}!", params.name).into())
                .unwrap();
            (state, response)
        }

        pub fn add(mut state: State) -> (State, Response<Body>) {
            let params = state.take::<AddParams>();
            let response = Response::builder()
                .status(StatusCode::OK)
                .body(format!("{} + {} = {}", params.x, params.y, params.x + params.y,).into())
                .unwrap();
            (state, response)
        }
    }

    mod resource {
        use super::*;
        pub fn create(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::CREATED)
                .body(Body::empty())
                .unwrap();
            (state, response)
        }

        pub fn destroy(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::ACCEPTED)
                .body(Body::empty())
                .unwrap();
            (state, response)
        }

        pub fn show(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::OK)
                .body("It's a resource.".into())
                .unwrap();
            (state, response)
        }

        pub fn update(state: State) -> (State, Response<Body>) {
            let response = Response::builder()
                .status(StatusCode::ACCEPTED)
                .body(Body::empty())
                .unwrap();
            (state, response)
        }
    }

    mod api {
        use super::*;
        pub fn submit(state: State) -> (State, Response<Body>) {
            (
                state,
                Response::builder()
                    .status(StatusCode::ACCEPTED)
                    .body(Body::empty())
                    .unwrap(),
            )
        }
    }

    #[test]
    fn build_router_test() {
        let pipelines = new_pipeline_set();
        let (pipelines, default) =
            pipelines.add(new_pipeline().add(NewSessionMiddleware::default()).build());

        let pipelines = finalize_pipeline_set(pipelines);

        let default_pipeline_chain = (default, ());

        let delegated_router = build_simple_router(|route| {
            route.get("/b").to(welcome::delegated);
        });

        let router = build_router(default_pipeline_chain, pipelines, |route| {
            route.get("/").to(welcome::index);

            route
                .get("/hello/:name")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::hello);

            route
                .get("/hello/:name/*")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::globbed);

            route
                .get("/goodbye/:name:[a-zA-Z]+")
                .with_path_extractor::<SalutationParams>()
                .to(welcome::goodbye);

            route
                .get("/add")
                .with_query_string_extractor::<AddParams>()
                .to(welcome::add);

            route.get(r"/literal/\:param/\*").to(welcome::literal);

            route.scope("/api", |route| {
                route.post("/submit").to(api::submit);
            });

            route.associate("/resource", |route| {
                route.post().to(resource::create);
                route.patch().to(resource::update);
                route.delete().to(resource::destroy);
                route.get_or_head().to(resource::show);
            });

            route.delegate("/delegated").to_router(delegated_router);
        });

        let new_service = GothamService::new(router);

        let call = move |req| {
            let mut service = new_service.connect("127.0.0.1:10000".parse().unwrap());
            service.call(req).wait().unwrap()
        };

        let response = call(Request::get("/").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);

        let response = call(Request::post("/api/submit").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = call(Request::get("/hello/world").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Hello, world!");

        let response = call(
            Request::get("/hello/world/more/path/here/handled/by/glob")
                .body(Body::empty())
                .unwrap(),
        );
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Globbed");

        let response = call(Request::get("/delegated/b").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "Delegated");

        let response = call(Request::get("/goodbye/world").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(
            &String::from_utf8(response_bytes).unwrap(),
            "Goodbye, world!"
        );

        let response = call(Request::get("/goodbye/9875").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = call(
            Request::get("/literal/:param/*")
                .body(Body::empty())
                .unwrap(),
        );
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = call(Request::get("/literal/a/b").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let response = call(Request::get("/add?x=16&y=71").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(&String::from_utf8(response_bytes).unwrap(), "16 + 71 = 87");

        let response = call(Request::post("/resource").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = call(Request::patch("/resource").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = call(Request::delete("/resource").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::ACCEPTED);

        let response = call(Request::get("/resource").body(Body::empty()).unwrap());
        assert_eq!(response.status(), StatusCode::OK);
        let response_bytes = response.into_body().concat2().wait().unwrap().to_vec();
        assert_eq!(&response_bytes[..], b"It's a resource.");
    }
}
