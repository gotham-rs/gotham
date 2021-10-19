#!/usr/bin/env bash
set -euo pipefail

# certificate authority
openssl ecparam -genkey -name prime256v1 -out tls_ca_key.pem
openssl req -batch -new -x509 -days 3650 -subj '/CN=Gotham Test CA' -extensions v3_ca -key tls_ca_key.pem -outform DER -out tls_ca_cert.der

# server certificate
openssl ecparam -genkey -name prime256v1 -outform DER | \
  openssl pkcs8 -topk8 -inform DER -nocrypt -outform DER -out tls_key.der
serial=$(calc 0x`openssl rand -hex 20`)
cat >tls_req.cnf <<EOF
[ext]
subjectAltName = @alt_names
[alt_names]
DNS.1=example.org
DNS.2=example.com
DNS.3=localhost
IP.1=127.0.0.1
IP.2=::1
EOF
openssl req -batch -new -subj '/CN=example.org' -keyform DER -key tls_key.der | \
  openssl x509 -req -days 3650 -CAform DER -CA tls_ca_cert.der -CAkey tls_ca_key.pem -extfile tls_req.cnf -extensions ext -set_serial $serial -outform DER -out tls_cert.der

# cleanup
rm tls_req.cnf
rm tls_ca_key.pem

# print certificates
echo
echo -e "\e[1mCA certificate:\e[0m"
openssl x509 -noout -text -inform DER -in tls_ca_cert.der
echo
echo -e "\e[1mServer certificate:\e[0m"
openssl x509 -noout -text -inform DER -in tls_cert.der
