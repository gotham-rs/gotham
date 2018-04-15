# Path extraction glob extraction

Shows how to match arbitrarily many path segments.

## Running

From the `examples/path/globs` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_path_globs v0.0.0 (file://.../gotham/examples/path/globs)
      Finished dev [unoptimized + debuginfo] target(s) in 2.36 secs
       Running `.../gotham/target/debug/gotham_examples_path_globs`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:

  $ curl -vvv http://localhost:7878/parts/heads/shoulders/knees/toes
  *   Trying 127.0.0.1...
  * TCP_NODELAY set
  * Connected to localhost (127.0.0.1) port 7878 (#0)
  > GET /parts/heads/shoulders/knees/toes HTTP/1.1
  > Host: localhost:7878
  > User-Agent: curl/7.54.0
  > Accept: */*
  > 
  < HTTP/1.1 200 OK
  < Content-Length: 39
  < Content-Type: text/plain
  < X-Request-ID: 3889c405-bb05-43f7-b717-c3eba2ed27fc
  < X-Frame-Options: DENY
  < X-XSS-Protection: 1; mode=block
  < X-Content-Type-Options: nosniff
  < X-Runtime-Microseconds: 165
  < Date: Mon, 19 Mar 2018 22:17:17 GMT
  < 
  Got 4 parts:
  heads
  shoulders
  knees
  * Connection #0 to host localhost left intact
  toes
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
