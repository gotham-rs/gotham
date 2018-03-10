# Cookies introduction

An introduction to storing and retrieving cookie data with the Gotham web framework.

## Running

From the `examples/cookies/introduction` directory:

```
Terminal 1:
  $ cargo run                                                                                                                                                                                                                     130 ↵
     Compiling gotham_examples_cookies_introduction v0.0.0 (file://.../gotham/examples/cookies/introduction)
      Finished dev [unoptimized + debuginfo] target(s) in 2.49 secs
       Running `.../gotham/target/debug/gotham_examples_cookies_introduction`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
  $ curl -v -c /tmp/cookiejar http://localhost:7878/
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
  < Content-Length: 25
  < Content-Type: text/plain
  < X-Request-ID: 1b0ace7a-8d6b-478b-a099-f3e57363d9cd
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  * Added cookie adjective="repeat" for domain localhost, path /, expire 0
  < Set-Cookie: adjective=repeat; HttpOnly
  < X-Runtime-Microseconds: 65
  < Date: Wed, 07 Mar 2018 00:05:27 GMT
  <
  Hello first time visitor
  * Connection #0 to host localhost left intact
  $ curl -v -b /tmp/cookiejar http://localhost:7878/
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
  > Cookie: adjective=repeat
  >
  < HTTP/1.1 200 OK
  < Content-Length: 21
  < Content-Type: text/plain
  < X-Request-ID: cc9c1e59-75ea-40e1-b511-98851c393cb9
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  * Replaced cookie adjective="repeat" for domain localhost, path /, expire 0
  < Set-Cookie: adjective=repeat; HttpOnly
  < X-Runtime-Microseconds: 113
  < Date: Wed, 07 Mar 2018 00:05:54 GMT
  <
  Hello repeat visitor
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
