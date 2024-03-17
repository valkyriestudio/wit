FROM registry.docker.com/library/rust:1.76.0 AS prep
WORKDIR /usr/src/myapp
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
   && apt-get -y install clang lld curl gnupg \
   && curl -sL https://deb.nodesource.com/setup_20.x | bash \
   && apt-get install nodejs -yq \
   && corepack enable yarn \
   && apt-get autoremove -y && apt-get clean -y

FROM prep AS build
COPY . .
RUN cargo build --release && cargo test --release

FROM registry.docker.com/library/debian:12-slim AS base
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
COPY --from=build /usr/src/myapp/target/release/wit /app
COPY --from=build /usr/src/myapp/assets /app/assets

CMD ["./wit"]
