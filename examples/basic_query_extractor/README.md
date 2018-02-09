# Basic Query String Extractor

A simple example of a query string extractor 

## Running

From the `examples/basic_query_extractor` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_query_extractor (file:///.../examples/basic_query_extractor)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../basic_query_extractor`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v 'http://localhost:7878/widgets?name=t-shirt'
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /widgets?name=t-shirt HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.58.0
> Accept: */*
> 
< HTTP/1.1 200 OK
< Content-Length: 31
< Content-Type: text/plain
< X-Request-ID: 09542fc7-6739-465c-9ee8-8aa18c3a5271
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 308
< Date: Wed, 07 Feb 2018 18:52:15 GMT
< 
* Connection #0 to host localhost left intact
{"name":"t-shirt","description":"t-shirt"}% 
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)