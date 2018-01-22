# Middleware Database

A simple introduction to working with Gotham.

## Running
For running this sample successfully you need to have a working connection to redis.

Modify the `"redis://localhost"` string on `src/config/middleware.rs` file if you have redis connection to other host.

From the `examples/middleware_database` directory:

```
Terminal 1:
$ cargo run
   Compiling middleware_database_runner v0.2.0 (file:///.../examples/middleware_database)
    Finished dev [unoptimized + debuginfo] target(s) in 6.16 secs
     Running `.../middleware_database_runner`
Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/database
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET /database HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.57.0
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 4
< Content-Type: text/plain
< X-Request-ID: 7a083779-b390-4992-a036-521da423e75f
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 469
< Date: Sat, 20 Jan 2018 14:02:28 GMT
<
* Connection #0 to host 127.0.0.1 left intact
PONG
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
