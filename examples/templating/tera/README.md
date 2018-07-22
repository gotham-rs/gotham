# Templating using Tera

An example usage of Tera template engine working with Gotham.

## Running

From the `examples/templating/tera` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_templating_tera (file:///.../examples/templating/tera)
    Finished dev [unoptimized + debuginfo] target(s) in 4.26 secs
     Running `../gotham_examples_templating_tera`
  Listening for requests at http://127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*  Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.1
> Accept: */*
>
< HTTP/1.1 200 OK
< Content-Length: 150
< Content-Type: text/html
< X-Request-ID: 181d26c4-ee9d-44ed-aa52-803b10560d71
< X-Frame-Options: DENY
< X-XSS-Protection: 1; mode=block
< X-Content-Type-Options: nosniff
< X-Runtime-Microseconds: 5437
< Date: Tue, 10 Jul 2018 15:37:58 GMT
<
<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <title>Gotham Tera example</title>
</head>
<body>
  <h1>Hello Gotham!</h1>
</body>
</html>
* Connection #0 to host 127.0.0.1 left intact
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
