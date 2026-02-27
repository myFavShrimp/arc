FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY . .

RUN cargo build --release

FROM scratch

COPY --from=builder /app/target/release/webservice /bin/webservice

EXPOSE 8080
ENTRYPOINT ["/bin/webservice"]
