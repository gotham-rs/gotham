#!/usr/bin/env bash

openssl req -x509 -newkey rsa -config ca.cfg -days 3650 -out ca_cert.pem
openssl req -newkey rsa -config srv.cfg -days 3650 |
  openssl x509 -days 3650 -req -CA ca_cert.pem -CAkey ca_key.pem -extfile srv.cfg -extensions v3_server -set_serial 1 -out cert.pem
rm ca_key.pem
echo "CA certificate:"
openssl x509 -noout -text < ca_cert.pem
echo
echo "Server certificate:"
openssl x509 -noout -text < cert.pem
