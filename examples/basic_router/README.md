# Basic Router

An example of the Gotham Router showing usage of HTTP verbs such as Get and Post.

## Running

From the `examples/basic_router` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_router (file:///.../examples/basic_router)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../basic_router`
  Accepting requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/checkout/start
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET /checkout/start HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 5
< Content-Type: text/plain
< X-Request-ID: 0f797cdf-5db5-4bee-82ae-ad8b12ab870c
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 119
< Date: Mon, 15 Jan 2018 04:04:13 GMT
<
* Connection #0 to host 127.0.0.1 left intact
start%

$ curl -d '' -v http://127.0.0.1:7878/checkout/complete
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> POST /checkout/complete HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
> Content-Length: 0
> Content-Type: application/x-www-form-urlencoded
>
< HTTP/1.1 200 OK
< Content-Length: 8
< Content-Type: text/plain
< X-Request-ID: 48b2cde0-2040-4bc4-a26c-503b01d4677d
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 71
< Date: Mon, 15 Jan 2018 04:03:38 GMT
<
* Connection #0 to host 127.0.0.1 left intact
complete%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
