# Basic IntoResponse Example 

A simple example implementing the `IntoResponse` trait for a `Product`.

## Running

From the `examples/basic_into_response` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_into_response (file:///.../examples/basic_into_response)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../basic_into_response`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://localhost:7878/widgets/t-shirt
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /widgets/t-shirt HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.58.0
> Accept: */*
> 
< HTTP/1.1 200 OK
< Content-Length: 31
< Content-Type: application/json
< X-Request-ID: c9a3d057-94c2-4922-8ffd-d483959b566f
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 218
< Date: Wed, 07 Feb 2018 19:46:33 GMT
< 
* Connection #0 to host localhost left intact
{"name":"t-shirt","price":15.5}%  
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
