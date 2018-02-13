# Basic Request Components 

An example showing the request components available.

## Running

From the `examples/basic_request_components` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_request_components (file:///.../examples/basic_request_components)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../basic_request_components`
Listening for requests at http://127.0.0.1:7878
Method: Get
URI: "/"
HTTP Version: Http11
Headers: {"Host": "localhost:7878", "User-Agent": "curl/7.58.0", "Accept": "*/*"}
Body: Body(Empty)

Terminal 2:
$ curl -v http://127.0.0.1:7878/
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
< Content-Length: 0
< Content-Type: text/plain
< X-Request-ID: e6cc269f-523d-4c1f-96c2-091cf9387315
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 197
< Date: Tue, 13 Feb 2018 19:42:02 GMT
< 
* Connection #0 to host localhost left intact
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
