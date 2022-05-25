FROM rust:1.59.0

WORKDIR /usr/src/app

COPY . .

EXPOSE 8081

RUN cargo build