extern crate futures;
extern crate futures_cpupool;
extern crate gotham;
#[macro_use]
extern crate gotham_derive;
extern crate hyper;
#[macro_use]
extern crate log;

#[cfg(test)]
extern crate mime;

mod job;
mod pool;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
