# Basic Diesel 

An example of the Gotham Diesel Middelware.

## Running

From the `examples/basic_diesel` directory:

```
Terminal 1:
$ cargo run
   Compiling basic_router (file:///.../examples/basic_diesel)
    Finished dev [unoptimized + debuginfo] target(s) in 3.32 secs
     Running `../basic_diesel`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878
* Rebuilt URL to: http://localhost:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.57.0
> Accept: */*
> 
< HTTP/1.1 200 OK
< X-Runtime-Microseconds: 754
< Transfer-Encoding: chunked
< Date: Thu, 01 Feb 2018 20:14:49 GMT
< 
* Connection #0 to host localhost left intact
[Post { id: Some(1), title: "test", body: "this a test post", published: true }, Post { id: Some(2), title: "another", body: "another post", published: true }]%     
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Conduct](../../CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)