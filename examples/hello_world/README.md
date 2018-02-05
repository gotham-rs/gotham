# Hello World

A simple introduction to working with Gotham.

## Running

From the `examples/hello_world` directory:

```
Terminal 1:
$ cargo run
   Compiling hello_world (file:///.../examples/hello_world)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../hello_world`
  Listening for requests at http://127.0.0.1:7878

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
< X-Request-ID: 88ec311c-fc77-4d2e-b302-b1ba38718d96
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 51
< Date: Fri, 05 Jan 2018 06:25:00 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello World!%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
