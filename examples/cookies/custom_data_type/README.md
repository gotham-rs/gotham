# Cookies introduction

An introduction to storing and retrieving session data with a custom data type, in a type safe way, with the Gotham web framework.

## Running

From the `examples/cookies/custom_data_type` directory:

```
Terminal 1:
  $ cargo run                                                                                                                                                                                                                     130 ↵
     Compiling gotham_examples_cookies_custom_data_type v0.0.0 (file://.../gotham/examples/cookies/custom_data_type)
      Finished dev [unoptimized + debuginfo] target(s) in 2.49 secs
       Running `.../gotham/target/debug/gotham_examples_cookies_custom_data_type`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v -c /tmp/cookiejar http://localhost:7878
  * Rebuilt URL to: http://localhost:7878/
  *   Trying ::1...
  * TCP_NODELAY set
  * Connection failed
  * connect to ::1 port 7878 failed: Connection refused
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET / HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.0
  > Accept: */*
  > 
  < HTTP/1.1 200 OK
  < Content-Length: 41
  < Content-Type: text/plain
  < X-Request-ID: 5fdd0a88-4b23-4c68-8f6b-91d3c6a69fd4
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  * Added cookie _gotham_session="op-CELe5R-mEJ3zxhcak4eElI5EUtslBLbZ6chyUvAWzEwkvAkPUBzsHj014xHW1tWq0RG4vyXSnXDZneqfxyA" for domain localhost, path /, expire 0
  < Set-Cookie: _gotham_session=op-CELe5R-mEJ3zxhcak4eElI5EUtslBLbZ6chyUvAWzEwkvAkPUBzsHj014xHW1tWq0RG4vyXSnXDZneqfxyA; HttpOnly; SameSite=Lax; Path=/
  < X-Runtime-Microseconds: 1143
  < Date: Thu, 01 Mar 2018 00:29:10 GMT
  < 
  You have never visited this page before.
  * Connection #0 to host localhost left intact

  $ curl -v -b /tmp/cookiejar http://localhost:7878
  * Rebuilt URL to: http://localhost:7878/
  *   Trying ::1...
  * TCP_NODELAY set
  * Connection failed
  * connect to ::1 port 7878 failed: Connection refused
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET / HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.0
  > Accept: */*
  > Cookie: _gotham_session=op-CELe5R-mEJ3zxhcak4eElI5EUtslBLbZ6chyUvAWzEwkvAkPUBzsHj014xHW1tWq0RG4vyXSnXDZneqfxyA
  > 
  < HTTP/1.1 200 OK
  < Content-Length: 87
  < Content-Type: text/plain
  < X-Request-ID: 0a202d40-2c89-418e-82b0-5854c0041665
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 400
  < Date: Thu, 01 Mar 2018 00:29:13 GMT
  < 
  You have visited this page 1 time(s) before. Your last visit was 2018-03-01T00:29:10Z.
  * Connection #0 to host localhost left intact

```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
