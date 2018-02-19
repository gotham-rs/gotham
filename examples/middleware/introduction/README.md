# Middleware and Pipelines introduction

Introduces the `Middleware` and `Pipelines` concepts provided by the
Gotham web framework.

## Running

From the `examples/middleware/introduction` directory:

```
Terminal 1:
  $ cargo run
     Compiling gotham_examples_functionality_name v0.0.0 (file://.../gotham/examples/middleware/introduction)
      Finished dev [unoptimized + debuginfo] target(s) in 2.56 secs
       Running `.../gotham/target/debug/gotham_examples_middleware_introduction`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
	$ curl -v http://localhost:7878/
	*   Trying 127.0.0.1...
	* TCP_NODELAY set
	* Connected to localhost (127.0.0.1) port 7878 (#0)
	> GET / HTTP/1.1
	> Host: localhost:7878
	> User-Agent: curl/7.54.1
	> Accept: */*
	>
	< HTTP/1.1 200 OK
	< Content-Length: 0
	< X-Request-ID: 22d760ad-a7d2-4cf1-accf-fb192230d0ea
	< X-Frame-Options: DENY
	< X-XSS-Protection: 1; mode=block
	< X-Content-Type-Options: nosniff
	< X-Stock-Remaining: 99
	< X-Runtime-Microseconds: 147
	< Date: Mon, 19 Feb 2018 10:29:41 GMT
	<
	* Connection #0 to host localhost left intact
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
