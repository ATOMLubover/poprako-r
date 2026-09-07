FROM rust:1.95.0-alpine3.22 AS builder

WORKDIR /work

# Keep optimized code and thin LTO, but allow parallel code generation.
# Each source build starts from the dependency layer, not a previous app build.
ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16

RUN apk add --no-cache \
    build-base \
    cmake \
    curl \
    libpq-dev \
    perl \
    pkgconf

# Only manifests belong before the dependency build. Workspace source, tests,
# and resources must not invalidate third-party compilation.
COPY Cargo.toml Cargo.lock ./
COPY poprako-util/Cargo.toml ./poprako-util/Cargo.toml
COPY poprako-swagger/Cargo.toml ./poprako-swagger/Cargo.toml
COPY poprako-obj-dept/Cargo.toml ./poprako-obj-dept/Cargo.toml
COPY poprako-obj-dept-macro/Cargo.toml ./poprako-obj-dept-macro/Cargo.toml
COPY poprako-rdb-core/Cargo.toml ./poprako-rdb-core/Cargo.toml

RUN mkdir -p src benches poprako-swagger/src && \
    printf 'fn main() {}\n' > src/main.rs && \
    printf '\n' > src/lib.rs && \
    printf 'fn main() {}\n' > benches/test_benchmark.rs && \
    printf 'fn main() {}\n' > poprako-swagger/src/main.rs && \
    for package in poprako-util poprako-obj-dept poprako-obj-dept-macro poprako-rdb-core; do \
        mkdir -p "$package/src" && printf '\n' > "$package/src/lib.rs" || exit 1; \
    done

# Persist registry sources with target artifacts in the exported registry layer.
# Cache mounts are local to the BuildKit builder on an ephemeral runner.
RUN cargo build --locked --release --bin poprako-server && \
    cargo clean --release \
        --package poprako-server \
        --package poprako-util \
        --package poprako-swagger \
        --package poprako-obj-dept \
        --package poprako-obj-dept-macro \
        --package poprako-rdb-core

# All stub artifacts were removed before this cached layer was exported.
# Real workspace code must compile even when checkout timestamps are older.
COPY poprako-util ./poprako-util
COPY poprako-swagger ./poprako-swagger
COPY poprako-obj-dept ./poprako-obj-dept
COPY poprako-obj-dept-macro ./poprako-obj-dept-macro
COPY poprako-rdb-core ./poprako-rdb-core
COPY benches ./benches
COPY src ./src

RUN cargo build --locked --release --bin poprako-server --timings && \
    cp /work/target/release/poprako-server /work/poprako-server

FROM alpine:3.22 AS runtime

WORKDIR /app

LABEL org.opencontainers.image.source="https://github.com/poprako-dev/poprako-server"

RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    libpq && \
    addgroup -S poprako && \
    adduser -S -G poprako -h /app poprako

COPY --from=builder --chown=poprako:poprako \
    /work/poprako-server /app/poprako-server
COPY --chown=poprako:poprako \
    deploy/poprako-server/app_config.toml /app/app_config.toml

USER poprako

EXPOSE 8888

HEALTHCHECK --interval=10s --timeout=3s --start-period=10s --retries=6 \
    CMD wget -q -O /dev/null http://127.0.0.1:8888/api/health || exit 1

CMD ["/app/poprako-server"]
