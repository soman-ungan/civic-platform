#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# 本番の backend のコンテナで、運営のコマンドを1つ動かす（docs/operations/cli.md）。
# 中身は `docker compose exec -T backend backend <引数>` を、サーバーの compose のフォルダで動かすだけ。
# 資料の取り込みは deploy/upload.sh を使う（原本と抽出の JSON を送ってから import する）
#
# 使い方:
#   deploy/prod.sh <動詞> <対象> [オプション...]
#
# 送り先（環境変数。deploy/upload.sh と同じ）:
#   DEPLOY_SSH  ssh に渡す接続先（~/.ssh/config のホストの別名がおすすめ。例: machikoe）。
#               空なら、このスクリプトを本番のサーバーの上で動かしているものとして、ssh を使わない
#   DEPLOY_DIR  サーバーで compose.yaml と .env を置いたフォルダ（例: ~/machikoe/deploy）。
#               DEPLOY_SSH が空なら、既定はこのスクリプトのあるフォルダ
#
# 例:
#   export DEPLOY_SSH=machikoe DEPLOY_DIR='~/machikoe/deploy'
#   deploy/prod.sh list topic -m otawara --drafts
#   deploy/prod.sh list usage --days 3
#   deploy/prod.sh publish budget -m otawara -y 2026

set -euo pipefail

usage() {
  sed -n '/^# 使い方:/,/^# 例:/p' "$0" | sed 's/^# \{0,1\}//' | sed '$d' >&2
  exit 2
}

[[ $# -ge 1 ]] || usage
case "$1" in -h|--help) usage ;; esac

if [[ -n "${DEPLOY_SSH:-}" ]]; then
  [[ -n "${DEPLOY_DIR:-}" ]] || { echo "DEPLOY_DIR を指定してください（サーバーの compose.yaml のフォルダ）" >&2; exit 2; }
  remote_dir="$DEPLOY_DIR"
else
  remote_dir="${DEPLOY_DIR:-$(cd "$(dirname "$0")" && pwd)}"
fi

# 引数を、サーバーのシェルにそのまま渡せる形にする
quote() { local out=() a; for a in "$@"; do out+=("$(printf '%q' "$a")"); done; echo "${out[*]}"; }

# DEPLOY_DIR の ~ はサーバーの側で展開する
cmd="cd $remote_dir && docker compose exec -T backend backend $(quote "$@")"
if [[ -n "${DEPLOY_SSH:-}" ]]; then
  # shellcheck disable=SC2086  # DEPLOY_SSH は ssh のオプションを含んでよい
  exec ssh $DEPLOY_SSH "$cmd"
else
  exec bash -c "$cmd"
fi
