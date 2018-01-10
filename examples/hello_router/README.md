# Hello Router

A Hello World example application for working with the Gotham Router.

## Running

From the `examples/hello_router` directory:

```
Terminal 1:
$ cargo run
   Compiling hello_router (file:///.../examples/hello_router)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../hello_router`
  Accepting requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*  Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 12
< Content-Type: text/plain
< X-Request-ID: 1f741585-ea29-4db9-8030-89b897bd2ada
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 51
< Date: Wed, 10 Jan 2018 10:47:40 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello Router!%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
