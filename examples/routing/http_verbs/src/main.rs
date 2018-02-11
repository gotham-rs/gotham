//! An example of the Gotham web framework Router that shows how to route requests to handlers
//! based on HTTP verbs.

extern crate futures;
extern crate gotham;
extern crate hyper;
extern crate mime;

use gotham::router::Router;
use gotham::router::builder::*;
use hyper::{Get, Head};

mod handlers;
use self::handlers::*;

/// Create a `Router`
///
/// Results in a tree of routes that that looks like:
///
/// /                        --> GET, HEAD
/// | products               --> GET, HEAD
/// | bag                    --> GET
/// | checkout/address       --> POST, PUT, PATCH, DELETE
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
        route.get_or_head("/products").to(products::index);
        route.get("/bag").to(bag::index);

        route
            .post("/checkout/address")
            .to(checkout::address::create);

        route.put("/checkout/address").to(checkout::address::update);

        route
            .patch("/checkout/address")
            .to(checkout::address::update);

        route
            .delete("/checkout/address")
            .to(checkout::address::delete);
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
    fn products_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/products")
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
}
