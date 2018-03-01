# Cookies introduction

An introduction to storing and retrieving session data, in a type safe way, with the Gotham web framework.

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
  < Content-Length: 44
  < Content-Type: text/plain
  < X-Request-ID: e9992f1d-9120-4473-be3e-60085098fb27
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  * Added cookie _gotham_session="rU4d0wvS1FO16jJ_pEzqrAYot6jcqrtpy8wDKEqwjuYbbgzunagwGA0h0kd6qH-cLwYlaGr3gOOxJEKmFa2pSg" for domain localhost, path /, expire 0
  < Set-Cookie: _gotham_session=rU4d0wvS1FO16jJ_pEzqrAYot6jcqrtpy8wDKEqwjuYbbgzunagwGA0h0kd6qH-cLwYlaGr3gOOxJEKmFa2pSg; HttpOnly; SameSite=Lax; Path=/
  < X-Runtime-Microseconds: 479
  < Date: Wed, 28 Feb 2018 23:16:05 GMT
  < 
  You have visited this page 0 time(s) before
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
  > Cookie: _gotham_session=rU4d0wvS1FO16jJ_pEzqrAYot6jcqrtpy8wDKEqwjuYbbgzunagwGA0h0kd6qH-cLwYlaGr3gOOxJEKmFa2pSg
  > 
  < HTTP/1.1 200 OK
  < Content-Length: 44
  < Content-Type: text/plain
  < X-Request-ID: 788c055a-293b-49da-986f-d0afed9015fb
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 320
  < Date: Wed, 28 Feb 2018 23:16:38 GMT
  < 
  You have visited this page 1 time(s) before
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
