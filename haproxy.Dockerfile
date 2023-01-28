FROM haproxy:2.7.2-alpine3.17
COPY haproxy.cfg /usr/local/etc/haproxy/haproxy.cfg
