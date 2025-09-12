FROM rust:1.82.0-bullseye as builder

RUN apt-get update && \
    apt-get install -y libopus-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /root/bot
COPY . .

RUN --mount=type=cache,target=/root/.cargo/bin \
    --mount=type=cache,target=/root/.cargo/registry/index \
    --mount=type=cache,target=/root/.cargo/registry/cache \
    --mount=type=cache,target=/root/.cargo/git/db \
    --mount=type=cache,target=/root/bot/target \
    cargo build --release --bin bot && \
    cp target/release/bot /usr/local/bin/bot

###

FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y ca-certificates ffmpeg && \
    rm -rf /var/lib/apt/lists/*

# Switch to unpriviledged user
RUN useradd --user-group bot
USER bot

COPY --from=builder /usr/local/bin/bot /usr/local/bin/bot

ARG SENTRY_RELEASE
ENV SENTRY_RELEASE=$SENTRY_RELEASE

ENTRYPOINT ["bot"]
