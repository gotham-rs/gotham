//! An example of serving static assets with Gotham.
use gotham::handler::assets::FileOptions;
use gotham::router::builder::{build_simple_router, DefineSingleRoute, DrawRoutes};

pub fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Need to pass an arg which is the path to serve");
    let addr = "127.0.0.1:7878";
    println!(
        "Listening for requests at http://{} from path {:?}",
        addr, path
    );

    let router = build_simple_router(|route| {
        // You can add a `to_dir` or `to_file` route simply using a
        // `String` or `str` as above, or a `Path` or `PathBuf` to accept
        // default options.
        route.get("/").to_file("assets/doc.html");
        // Or you can customize options for comressed file handling, cache
        // control headers etc by building a `FileOptions` instance.
        route.get("assets/*").to_dir(
            FileOptions::new(&path)
                .with_cache_control("no-cache")
                .with_gzip(true)
                .build(),
        );
    });

    gotham::start(addr, router)
}
