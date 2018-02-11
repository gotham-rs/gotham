# Basic Path Extractor 

A Path Extractor example application for working with the Gotham Path Extractor

## Running

From the `examples/basic_path_extractor` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_path_extractor (file:///.../examples/basic_path_extractor)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../basic_path_extractor`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
curl -v http://localhost:7878/widgets/t-shirt
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /widgets/t-shirt HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.58.0
> Accept: */*
> 
< HTTP/1.1 200 OK
< Content-Length: 16
< Content-Type: text/plain
< X-Request-ID: 2f2beb32-58e5-43b8-a27a-a0b61d1792f0
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 214
< Date: Tue, 06 Feb 2018 20:14:14 GMT
< 
* Connection #0 to host localhost left intact
Product: t-shirt%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
