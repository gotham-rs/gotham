//! A basic example application for working with the Gotham Router.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use gotham::router::Router;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};
use hyper::{Get, Head};

mod handlers;
use self::handlers::*;

/// Create a `Router`
///
/// Results in a tree of routes that that looks like:
///
/// /                     --> GET, HEAD
/// | widgets             --> GET, HEAD
/// | bag                 --> GET
/// | checkout
///   | start             --> GET
///   | address           --> POST, PUT, PATCH, DELETE
///   | payment_details   --> POST, PUT
///   | complete          --> POST
/// | api
///   | widgets           --> GET
///
/// If no match for a request is found a 404 will be returned. Both the HTTP verb and the request
/// path are considered when determining if the request matches a defined route.
///
/// The API documentation for `DrawRoutes` describes all the HTTP verbs which Gotham is capable of
/// matching on.
fn router() -> Router {
    build_simple_router(|route| {
        // get_or_head is valid here, `request` used simply as API example
        route.request(vec![Get, Head], "/").to(index);
        route.get_or_head("/widgets").to(widgets::index);
        route.get("/bag").to(bag::index);

        route.scope("/checkout", |route| {
            route.get("/start").to(checkout::start);

            route.associate("/address", |assoc| {
                assoc.post().to(checkout::address::create);
                assoc.put().to(checkout::address::update);
                assoc.patch().to(checkout::address::update);
                assoc.delete().to(checkout::address::delete);
            });

            route
                .post("/payment_details")
                .to(checkout::payment_details::create);
            route
                .put("/payment_details")
                .to(checkout::payment_details::update);

            route.post("/complete").to(checkout::complete);
        });

        route.scope("/api", |route| {
            route.get("/widgets").to(api::widgets::index);
        });
    })
}

/// Start a server and use a `Router` to dispatch requests
pub fn main() {
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{}", addr);
    gotham::start(addr, router())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gotham::test::TestServer;
    use hyper::StatusCode;

    #[test]
    fn index_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"index");
    }

    #[test]
    fn index_delete() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .delete("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::MethodNotAllowed);
    }

    #[test]
    fn index_head() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .head("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);
        assert!(response.read_body().unwrap().is_empty());
    }

    #[test]
    fn widgets_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/widgets")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"index");
    }

    #[test]
    fn bag_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/bag")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"index");
    }

    #[test]
    fn checkout_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/checkout/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::NotFound);
    }

    #[test]
    fn checkout_start_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/checkout/start")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"start");
    }

    #[test]
    fn checkout_complete_post() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost/checkout/complete",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"complete");
    }

    #[test]
    fn checkout_address_post() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost/checkout/address",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"create");
    }

    #[test]
    fn checkout_address_put() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .put(
                "http://localhost/checkout/address",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"update");
    }

    #[test]
    fn checkout_address_patch() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .patch(
                "http://localhost/checkout/address",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"update");
    }

    #[test]
    fn checkout_address_delete() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .delete("http://localhost/checkout/address")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"delete");
    }

    #[test]
    fn checkout_payment_details_post() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post(
                "http://localhost/checkout/payment_details",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"create");
    }

    #[test]
    fn checkout_payment_details_put() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .put(
                "http://localhost/checkout/payment_details",
                "data",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"update");
    }

    #[test]
    fn api_widgets_delete() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/api/widgets")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Ok);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"index");
    }
}
