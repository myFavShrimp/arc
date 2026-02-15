FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev pkgconf openssl-dev openssl-libs-static

WORKDIR /app
COPY . .

RUN cargo build --release

FROM alpine AS files

RUN adduser -D -H -u 10001 appuser
RUN apk add --no-cache ca-certificates

FROM scratch

COPY --from=files /etc/passwd /etc/passwd
COPY --from=files /etc/group /etc/group
COPY --from=files /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /app/target/release/webservice /bin/webservice

USER appuser
EXPOSE 8080
ENTRYPOINT ["/bin/webservice"]
