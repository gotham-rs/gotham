# Path extraction using Regex

Shows how to match against Regex patterns in path segments.

## Running

From the `examples/path/regex` directory:

```
Terminal 1:

   Compiling gotham_examples_path_regex v0.0.0 (file://.../examples/path/regex)
    Finished dev [unoptimized + debuginfo] target(s) in 2.57 secs
     Running `.../target/debug/gotham_examples_path_regex`
Listening for requests at http://127.0.0.1:7878

Terminal 2:

  $ curl -v http://127.0.0.1:7878/user/123
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
  > GET /user/123 HTTP/1.1
  > Host: 127.0.0.1:7878
  > User-Agent: curl/7.54.0
  > Accept: */*
  >
  < HTTP/1.1 200 OK
  < Content-Length: 16
  < Content-Type: text/plain
  < X-Request-ID: a678159d-d1d8-4cf5-92e9-675cb581b347
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 322
  < Date: Thu, 19 Apr 2018 23:51:33 GMT
  <
  * Connection #0 to host 127.0.0.1 left intact
  Hello, User 123!

  $ curl -v http://127.0.0.1:7878/user/abc
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
  > GET /user/abc HTTP/1.1
  > Host: 127.0.0.1:7878
  > User-Agent: curl/7.54.0
  > Accept: */*
  >
  < HTTP/1.1 404 Not Found
  < Content-Length: 0
  < X-Request-ID: 58c302a8-80d7-4b6a-8d7c-5a01b5c587c1
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 264
  < Date: Thu, 19 Apr 2018 23:52:12 GMT
  <
  * Connection #0 to host 127.0.0.1 left intact
```

## License

Licensed under your option of:

* [MIT License](../../../LICENSE-MIT)
* [Apache License, Version 2.0](../../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../../CODE_OF_CONDUCT.md)
* [Contributing](../../../CONTRIBUTING.md)
