# Diesel Example

An example of the Gotham Diesel Middelware.

The gotham diesel middleware uses `tokio_threadpool::blocking`, which allows
blocking operations to run without blocking the tokio reactor. Although not true async,
this allows multiple concurrent database requests to be handled, with a default of 100
concurrent blocking operations. For further details see
[https://docs.rs/tokio-threadpool/0.1.8/tokio_threadpool/fn.blocking.html](tokio_threadpool::blocking documentation).

## Running tests
Diesel will expect the tests database to exist before running tests.
You can initialise with
```
DATABASE_URL=tests.db diesel database setup
```
and then run tests:
```
cargo test
```
The cargo tests in this repo embed the migrations to apply them automatically, however you could
also do this externally using a tool like [cargo-make](https://sagiegurari.github.io/cargo-make/)

## Running

From the `examples/diesel` directory:

```
Terminal 1:
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.61 secs
     Running `../gotham_diesel_example`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.64.0
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 62d29d71-ab89-4d9e-a91d-77b22ae3c6dc
< content-type: application/json
< content-length: 2
< date: Thu, 11 Apr 2019 13:52:06 GMT
<
* Connection #0 to host 127.0.0.1 left intact
[]%

$ curl -v -H "Content-Type: application/json" -d '{"title":"test","price":1.0,"link":"http://localhost"}' 'http://localhost:7878'
*   Trying ::1...
* TCP_NODELAY set
* connect to ::1 port 7878 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> POST / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.64.0
> Accept: */*
> Content-Type: application/json
> Content-Length: 54
>
* upload completely sent off: 54 out of 54 bytes
< HTTP/1.1 201 Created
< x-request-id: d02d724a-4b88-4aac-9da3-adef60fff258
< content-type: application/json
< content-length: 10
< date: Thu, 11 Apr 2019 13:52:40 GMT
<
* Connection #0 to host localhost left intact
{"rows":1}%

$ curl -v localhost:7878
*   Trying ::1...
* TCP_NODELAY set
* connect to ::1 port 7878 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.64.0
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 28a3cd70-d781-4671-b52f-67d096e38a79
< content-type: application/json
< content-length: 63
< date: Thu, 11 Apr 2019 13:54:10 GMT
<
* Connection #0 to host localhost left intact
[{"id":1,"title":"test","price":1.0,"link":"http://localhost"}]%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)