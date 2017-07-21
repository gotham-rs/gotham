extern crate gotham;
extern crate hyper;
extern crate futures;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate fern;
extern crate log;
extern crate chrono;

mod apps;

use log::LogLevelFilter;
use hyper::server::Http;
use gotham::handler::NewHandlerService;

use apps::todo::boot::router::router;

pub fn start() {
    fern::Dispatch::new()
        .level(LogLevelFilter::Error)
        .level_for("gotham", log::LogLevelFilter::Error)
        .level_for("gotham::state", log::LogLevelFilter::Error)
        .level_for("todo_session", log::LogLevelFilter::Error)
        .chain(std::io::stdout())
        .format(|out, message, record| {
                    out.finish(format_args!("{}[{}][{}]{}",
                                            chrono::UTC::now().format("[%Y-%m-%d %H:%M:%S%.9f]"),
                                            record.target(),
                                            record.level(),
                                            message))
                })
        .apply()
        .unwrap();

    let addr = "127.0.0.1:7878".parse().unwrap();

    let server = Http::new()
        .bind(&addr, NewHandlerService::new(router()))
        .unwrap();

    println!("Listening on http://{} with 1 thread.",
             server.local_addr().unwrap());
    server.run().unwrap();
}
