extern crate gotham;
extern crate middleware_database;

fn main() {
    let (address, router) = middleware_database::init().unwrap();
    println!("Listening for requests at http://{}", address);
    gotham::start(address, router)
}
