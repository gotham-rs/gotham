# Hello World

A simple introduction to working with TLS in Gotham.

## Running

From the `examples/openssl` directory:

```
Terminal 1:
$ cargo run
    Finished dev [unoptimized + debuginfo] target(s) in 0.24s
     Running `/home/judson/dev/gotham/target/debug/gotham_examples_hello_world_tls`
Listening for requests at https://127.0.0.1:7878

Terminal 2:
$ curl -v --cacert ca_cert.pem 'https://127.0.0.1:7878'
* Rebuilt URL to: https://127.0.0.1:7878/
*   Trying 127.0.0.1...
* TCP_NODELAY set
* Connected to 127.0.0.1 (127.0.0.1) port 7878 (#0)
* ALPN, offering h2
* ALPN, offering http/1.1
* Cipher selection: ALL:!EXPORT:!EXPORT40:!EXPORT56:!aNULL:!LOW:!RC4:@STRENGTH
* successfully set certificate verify locations:
*   CAfile: ca_cert.pem
  CApath: none
* TLSv1.2 (OUT), TLS handshake, Client hello (1):
* TLSv1.2 (IN), TLS handshake, Server hello (2):
* TLSv1.2 (IN), TLS handshake, Certificate (11):
* TLSv1.2 (IN), TLS handshake, Server key exchange (12):
* TLSv1.2 (IN), TLS handshake, Server finished (14):
* TLSv1.2 (OUT), TLS handshake, Client key exchange (16):
* TLSv1.2 (OUT), TLS change cipher, Client hello (1):
* TLSv1.2 (OUT), TLS handshake, Finished (20):
* TLSv1.2 (IN), TLS change cipher, Client hello (1):
* TLSv1.2 (IN), TLS handshake, Finished (20):
* SSL connection using TLSv1.2 / ECDHE-RSA-AES128-GCM-SHA256
* ALPN, server did not agree to a protocol
* Server certificate:
*  subject: C=US; ST=New York; L=Gotham; O=Gotham; OU=WWW-testing; CN=example.com
*  start date: Mar 30 19:54:02 2019 GMT
*  expire date: Mar 27 19:54:02 2029 GMT
*  subjectAltName: host "127.0.0.1" matched cert's IP address!
*  issuer: C=US; ST=New York; L=Gotham; O=Gotham; OU=WWW; CN=Gotham Test CA
*  SSL certificate verify ok.
> GET / HTTP/1.1
> Host: 127.0.0.1:7878
> User-Agent: curl/7.54.0
> Accept: */*
>
< HTTP/1.1 200 OK
< x-request-id: 56827b8c-ffcf-4b62-bb32-3440b17c93ee
< content-type: text/plain
< content-length: 12
< date: Mon, 09 Sep 2019 08:38:32 GMT
<
* Connection #0 to host 127.0.0.1 left intact
Hello World!
```

## License

Licensed under your option of:

* [MIT License](../../LICENSE-MIT)
* [Apache License, Version 2.0](../../LICENSE-APACHE)

## Community

The following policies guide participation in our project and our community:

* [Code of conduct](../../CODE_OF_CONDUCT.md)
* [Contributing](../../CONTRIBUTING.md)
