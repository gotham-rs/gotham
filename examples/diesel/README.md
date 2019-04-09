# Diesel Example

An example of the Gotham Diesel Middelware.

## Running

From the `examples/diesel` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_diesel (file:///.../examples/diesel)
    Finished dev [unoptimized + debuginfo] target(s) in 3.32 secs
     Running `../basic_diesel`
  Listening for requests at http://127.0.0.1:7878
Terminal 2:
$ curl -v http://127.0.0.1:7878
* Rebuilt URL to: localhost:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.57.0
> Accept: */*
>
< HTTP/1.1 200 OK
< X-Runtime-Microseconds: 12604
< Transfer-Encoding: chunked
< Date: Fri, 02 Feb 2018 21:42:14 GMT
<
* Connection #0 to host localhost left intact
[]%
$ curl -v -H "Content-Type: application/json" -X POST -d '{"title":"test","price":1.0,"link":"http://localhost"}' 'http://localhost:7878'
* Rebuilt URL to: http://localhost:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> POST / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.57.0
> Accept: */*
> Content-Type: application/json
> Content-Length: 47
>
* upload completely sent off: 47 out of 47 bytes
< HTTP/1.1 201 Created
< Content-Length: 0
< Content-Type: text/plain
< X-Request-ID: fcfcb0e2-604a-4070-8f5c-97ba1e729888
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 9510
< Date: Fri, 02 Feb 2018 21:42:34 GMT
<
* Connection #0 to host localhost left intact
$ curl -v localhost:7878
* Rebuilt URL to: localhost:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.58.0
> Accept: */*
>
< HTTP/1.1 200 OK
< X-Runtime-Microseconds: 427
< Transfer-Encoding: chunked
< Date: Tue, 06 Feb 2018 18:54:12 GMT
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