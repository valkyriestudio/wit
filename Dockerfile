FROM rust:1.76.0-slim AS prep
WORKDIR /usr/src/myapp
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld \
   && apt-get autoremove -y && apt-get clean -y

FROM prep AS build
COPY . .
RUN cargo build --release

FROM debian:slim AS base
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
COPY --from=build /usr/src/myapp/target/release/wit /app

CMD ["./wit"]
