# Multiple pipelines

Provides examples of using multiple middleware pipelines for different routes.

## Running

From the `examples/middleware/multiple-pipelines` directory:

Run the example app in one terminal:
Terminal 1:
```
  $ cargo run
   Compiling gotham_examples_middleware_multiple_pipelines v0.0.0 (file:///home/colin/code/gotham/gotham/examples/middleware/multiple_pipelines)
    Finished dev [unoptimized + debuginfo] target(s) in 12.67s
     Running `/home/colin/code/gotham/gotham/target/debug/gotham_examples_middleware_multiple_pipelines`
Listening for requests at http://127.0.0.1:7878
```

Our home page should not try and set a cookie:
Terminal 2:
```
$curl -v localhost:7878/
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET / HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.61.1
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 390559d5-c5a1-4859-bd36-058571767cbd
< content-type: text/html
< content-length: 168
< date: Tue, 06 Nov 2018 14:18:36 GMT
<

    <html>
    <head>Gotham</head>
    <body>
        <p>A flexible web framework that promotes stability, safety, security and speed.</p>
    </body>
    </html>
* Connection #0 to host localhost left intact
```

Our 'account' path includes a session middleware which does set a cookie:
```
$curl -v localhost:7878/account
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /account HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.61.1
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 8ab2a1bc-dd48-455c-a35c-7d3224ab6d53
< content-type: text/html
< set-cookie: _gotham_session=e-xnwZpilw25KLpNdeK2ZYRws4sXVnhnDObj9mkcOTl7NT7KE-0RuFVF-Mz-nfe1QGqzE_qlce3gZdRPIuHxdA; Secure; HttpOnly; SameSite=Lax; Path=/
< content-length: 168
< date: Tue, 06 Nov 2018 14:19:32 GMT
<

    <html>
    <head>Gotham</head>
    <body>
        <p>A flexible web framework that promotes stability, safety, security and speed.</p>
    </body>
    </html>
* Connection #0 to host localhost left intact
```

Our 'admin' path sets 2 cookies - from the default and the admin session middleware.
```
$curl -v localhost:7878/admin
*   Trying ::1...
* TCP_NODELAY set
* connect to ::1 port 7878 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /admin HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.61.1
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 4bbf5e10-5693-4724-aae3-c9d2ff1fcdfd
< content-type: text/html
< set-cookie: _gotham_session=hpkkIYi5L0x6FaOAkUkezFBN5SlL7AHsqC77NyK1ROJto_dsDSGayFni4x1NC_kQscfDWJpHFulV2XLHtB_0ew; Secure; HttpOnly; SameSite=Lax; Path=/
< set-cookie: _gotham_session=2XmDf4uA1in3CCTRDa5eQTMX14Xd8Wz52hRUaI2ksgpKqY1eiMgUUYOUM4V4LUy5mRORjM4H6kjvQAZ8HumCGw; Secure; HttpOnly; SameSite=Lax; Path=/
< content-length: 168
< date: Wed, 07 Nov 2018 03:16:30 GMT
<

    <html>
    <head>Gotham</head>
    <body>
        <p>A flexible web framework that promotes stability, safety, security and speed.</p>
    </body>
    </html>
* Connection #0 to host localhost left intact
```

Our 'api' path can return JSON:
```
curl -v -H "Accept: application/json" localhost:7878/api
*   Trying ::1...
* TCP_NODELAY set
* connect to ::1 port 7878 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /api HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.61.1
> Accept: application/json
>
< HTTP/1.1 200 OK
< x-request-id: dee26ec6-f475-47b0-ad2a-f35c8834790a
< content-type: application/json
< content-length: 105
< date: Wed, 07 Nov 2018 03:23:06 GMT
<
{
        "Gotham": "A flexible web framework that promotes stability, safety, security and speed."
* Connection #0 to host localhost left intact
    }%
```

But rejects requests that want XML:
```
curl -v -H "Accept: text/xml" localhost:7878/api
*   Trying ::1...
* TCP_NODELAY set
* connect to ::1 port 7878 failed: Connection refused
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to localhost (127.0.0.1) port 7878 (#0)
> GET /api HTTP/1.1
> Host: localhost:7878
> User-Agent: curl/7.61.1
> Accept: text/xml
>
< HTTP/1.1 400 Bad Request
< x-request-id: 4ba5a739-fa7e-4fcb-9a46-93ae24d49eb1
< content-type: application/json
< content-length: 33
< date: Wed, 07 Nov 2018 03:23:21 GMT
<
* Connection #0 to host localhost left intact
{"message":"Invalid accept type"}%
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
