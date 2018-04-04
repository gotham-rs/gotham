//! An example of serving static files with Gotham.

extern crate gotham;
extern crate env_logger;

use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};

pub fn main() {
    env_logger::init();
    let path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| panic!("Need to pass an arg which is path to serve"),
    );
    let addr = "127.0.0.1:7878";
    println!("Listening for requests at http://{} from path {:?}", addr, path);

    let router =
        build_simple_router(|route| {
            route.get("/*").to_filesystem(&path);
            route.get("/").to_file("assets/doc.html")
        });

    gotham::start(addr, router)
}