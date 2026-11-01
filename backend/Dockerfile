# syntax=docker/dockerfile:1
# SPDX-License-Identifier: AGPL-3.0-only
#
# backend（API サーバーと運営のコマンド）の本番イメージ。リポジトリのルートを context にして作る:
#   docker build -f backend/Dockerfile .
# マイグレーションはバイナリに埋め込み済み（sqlx::migrate!）。イメージに入れるのは data/seeds/ だけ
# （evals・docs・テスト・開発用の seed は入れない。docs/plans/2026-11-01-phase7-release.md）

FROM rust:1.99-slim-bookworm AS build
# パスキー（webauthn-rs）が OpenSSL を使う
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src/backend
COPY backend/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/backend/target \
    cargo build --release --locked && cp target/release/backend /usr/local/bin/backend

FROM debian:bookworm-slim
# ca-certificates: 外部の HTTPS（Claude API・決算カードの取得）のため。libssl3: パスキー（webauthn-rs）
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home /data app \
    && mkdir -p /data && chown app /data
COPY --from=build /usr/local/bin/backend /usr/local/bin/backend
COPY data/seeds /app/seeds
ENV BIND_ADDR=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/civic.db?mode=rwc \
    SEEDS_DIR=/app/seeds \
    DEV_SEEDS_DIR=/nonexistent \
    PRIVATE_DIR=/data/private
USER app
VOLUME /data
EXPOSE 8080
ENTRYPOINT ["backend"]
