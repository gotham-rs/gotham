//!  An example of the Gotham web framework `Router` that shows how to combine `Routes`
//!  under a common root using scopes.
use gotham::hyper::Method;
use gotham::router::builder::*;
use gotham::router::Router;

mod handlers;
use self::handlers::*;

/// Create a `Router`
///
/// Results in a tree of routes that that looks like:
///
/// /                     --> GET, HEAD
/// | products            --> GET, HEAD
/// | bag                 --> GET
/// | checkout
///   | start             --> GET
///   | address           --> POST, PUT, PATCH, DELETE
///   | payment_details   --> POST, PUT
///   | complete          --> POST
/// | api
///   | products           --> GET
///
/// If no match for a request is found a 404 will be returned. Both the HTTP verb and the request
/// path are considered when determining if the request matches a defined route.
///
/// The API documentation for `DrawRoutes` describes all the HTTP verbs which Gotham is capable of
/// matching on.
fn router() -> Router {
    build_simple_router(|route| {
        route
            .request(vec![Method::GET, Method::HEAD], "/")
            .to(index);
        route.get_or_head("/products").to(products::index);
        route.get("/bag").to(bag::index);

        // Scopes collect multiple paths under a common root node.
        // Scopes can define additional scopes for flexibility in your routing tree design.
        route.scope("/checkout", |route| {
            route.get("/start").to(checkout::start);

            route.post("/address").to(checkout::address::create);
            route.put("/address").to(checkout::address::update);
            route.patch("/address").to(checkout::address::update);
            route.delete("/address").to(checkout::address::delete);

            route
                .post("/payment_details")
                .to(checkout::payment_details::create);

            route
                .put("/payment_details")
                .to(checkout::payment_details::update);

            route.post("/complete").to(checkout::complete);
        });

        route.scope("/api", |route| {
            route.get("/products").to(api::products::index);
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
    use gotham::hyper::StatusCode;
    use gotham::test::TestServer;

    #[test]
    fn index_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn index_head() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .head("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.read_body().unwrap().is_empty());
    }

    #[test]
    fn widgets_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/products")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn checkout_start_get() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/checkout/start")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

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

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"update");
    }

    #[test]
    fn api_widgets_delete() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/api/products")
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"index");
    }
}
