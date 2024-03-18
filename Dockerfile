FROM registry.docker.com/library/rust:1.76.0 AS prep
WORKDIR /usr/src/myapp
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld \
   && apt-get autoremove -y && apt-get clean -y

FROM prep AS build
COPY . .
RUN ./assets/bundle.sh && ls -hl assets && cargo build --release && cargo test --release

FROM registry.docker.com/library/debian:12-slim AS base
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
COPY --from=build /usr/src/myapp/target/release/wit /app

CMD ["./wit"]
