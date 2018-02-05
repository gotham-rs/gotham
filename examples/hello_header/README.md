# Hello Header

A simple introduction to working with Gotham and custom headers.

## Running

From the `examples/hello_header` directory:

```
Terminal 1:
$ cargo run
   Compiling hello_header (file:///.../examples/hello_header)
    Finished dev [unoptimized + debuginfo] target(s) in 4.31 secs
     Running `../hello_header`
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
< Content-Length: 0
< Content-Type: text/plain
< X-Request-ID: 6ebf9a41-dc47-43ef-b431-eb0640b596b4
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Gotham: Hello World!
< X-Runtime-Microseconds: 41
< Date: Mon, 05 Feb 2018 01:11:01 GMT
<
* Connection #0 to host 127.0.0.1 left intact
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
