# Setting header values

Shows how to set a header value in a Gotham web framework response.

## Running

From the `examples/headers/setting` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_headers_setting v0.0.0 (file://.../gotham/examples/headers/setting)
      Finished dev [unoptimized + debuginfo] target(s) in 2.56 secs
       Running `.../gotham/target/debug/gotham_examples_headers_setting`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v http://127.0.0.1:7878
  * Rebuilt URL to: http://127.0.0.1:7878/
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
  > GET / HTTP/1.1
  > Host: 127.0.0.1:7878
  > User-Agent: curl/7.54.1
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 0
  < X-Request-ID: 2f8a11cb-7fff-4fe0-a64d-ca8f8d1a625d
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Gotham: Hello World!
  < X-Runtime-Microseconds: 80
  < Date: Fri, 16 Feb 2018 10:57:10 GMT
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
