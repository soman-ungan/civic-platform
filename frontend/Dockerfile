# syntax=docker/dockerfile:1
# SPDX-License-Identifier: AGPL-3.0-only
#
# frontend（Next.js）の本番イメージ。リポジトリのルートを context にして作る:
#   docker build -f frontend/Dockerfile --build-arg NEXT_PUBLIC_SOURCE_URL=... .
# NEXT_PUBLIC_* はビルドのときに埋め込まれる（公開してよい値だけ）。運営者ごとの値（SITE_*）と
# backend の URL（API_INTERNAL_URL）は、起動したときの環境変数で渡す（イメージに焼き込まない）

FROM node:24-bookworm-slim AS build
WORKDIR /src/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY frontend/ ./
# AGPL 第13条: フッターのソースへのリンク（必須）と、動いているコミット
ARG NEXT_PUBLIC_SOURCE_URL
ARG NEXT_PUBLIC_SITE_URL=""
ARG NEXT_PUBLIC_GIT_SHA=""
ENV NEXT_PUBLIC_SOURCE_URL=$NEXT_PUBLIC_SOURCE_URL \
    NEXT_PUBLIC_SITE_URL=$NEXT_PUBLIC_SITE_URL \
    NEXT_PUBLIC_GIT_SHA=$NEXT_PUBLIC_GIT_SHA \
    NEXT_TELEMETRY_DISABLED=1
RUN npm run build

FROM node:24-bookworm-slim
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    HOSTNAME=0.0.0.0 \
    PORT=3000
COPY --from=build --chown=node /src/frontend/.next/standalone ./
COPY --from=build --chown=node /src/frontend/.next/static ./.next/static
USER node
EXPOSE 3000
CMD ["node", "server.js"]
