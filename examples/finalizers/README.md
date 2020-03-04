# Finalizer example

How to add a finalizer that runs just before the response is returned, e.g. 
for 404 handling.

## Running

From the `examples/finalizers` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_finalizer v0.0.0 (/ssd/upstream/gotham/examples/finalizers)
    Finished dev [unoptimized + debuginfo] target(s) in 6.00s
     Running `/ssd/upstream/gotham/target/debug/gotham_examples_finalizer`
Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/no_such_path
*   Trying 127.0.0.1:7878...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET /no_such_path HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.65.3
> Accept: */*
>
* Mark bundle as not supporting multiuse
< HTTP/1.1 404 Not Found
< x-request-id: 3853acd7-e2b6-4a21-92a0-c33c403d4f28
< content-length: 32
< date: Wed, 04 Mar 2020 12:20:33 GMT
<
* Connection #0 to host 127.0.0.1 left intact
The status code is 404 Not Found
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
