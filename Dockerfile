FROM docker.io/library/rust:1.83 AS server
WORKDIR /app
COPY .rustfmt.toml Cargo.lock Cargo.toml ./
COPY wit ./wit
RUN cargo fetch
RUN cargo test \
    && cargo build --release

FROM docker.io/library/node:22 AS assets
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY wit ./wit
RUN corepack enable \
    && pnpm install --frozen-lockfile \
    && pnpm build

FROM docker.io/library/debian:12-slim AS base
RUN groupadd --gid 1000 wit \
    && useradd --uid 1000 --gid wit --shell /bin/bash --create-home wit
USER wit
WORKDIR /app
COPY --chown=1000:1000 --from=server /app/target/release/wit /app
COPY --chown=1000:1000 --from=assets /app/assets /app/assets
COPY --chown=1000:1000 hack/.gitconfig /home/wit
RUN mkdir -p repo \
    && ls -hl assets

ENV WIT_REPO_ROOT=/app/repo

EXPOSE 3000

CMD ["./wit"]
