# Templating using Askama

An example usage of the Askama template engine working with Gotham.

## Running

From the `examples/templating/askama` directory:

```
Terminal 1:
$ cargo run
   Compiling gotham_examples_templating_askama (file:///.../examples/templating/tera)
    Finished dev [unoptimized + debuginfo] target(s) in 13.18s
     Running `../gotham_examples_templating_askama`
  Listening at 127.0.0.1:7878

Terminal 2:
$ curl -v http://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.0
> Accept: */*
> 
< HTTP/1.1 200 OK
< x-request-id: fc67c641-e586-4f79-a972-6171efde6782
< content-type: text/html; charset=utf-8
< content-length: 165
< date: Sat, 08 Dec 2018 21:04:32 GMT
< 
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Hello, Gotham!</title>
</head>
<body>
    Askama says: "Hello, Gotham!"
</body>
* Connection #0 to host 127.0.0.1 left intact
</html>
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
