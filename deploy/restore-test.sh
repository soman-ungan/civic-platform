#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# バックアップから戻せることを確かめる（docs/operations/deploy.md「戻せることを確かめる」）。サーバーの上で動かす。
#   bash ~/machikoe/deploy/restore-test.sh <自治体の slug>...
# 1. 中身の一覧を控える 2. 止めて DB をボリュームの外に写す 3. ボリュームを消して起動（restore が R2 から戻す）4. 一覧を比べる
# 本番は数分止まる。途中で失敗したら起動し直す。
# ssh で 'bash -s' < このファイル のように標準入力で渡さない（docker compose が残りを読んでしまう）
set -euo pipefail
exec </dev/null
[ $# -ge 1 ] || { echo "使い方: $0 <自治体の slug>..." >&2; exit 2; }
cd "$(dirname "$0")"
trap 'echo "失敗したので起動し直します"; docker compose up -d' ERR

fp() {
  for slug in "$@"; do
    for t in topic minutes budget settlement tag source; do
      docker compose exec -T backend backend list "$t" -m "$slug" 2>/dev/null
    done
  done
  docker compose exec -T backend backend list operator 2>/dev/null
}

echo "== 1. 今の一覧を控える"
fp "$@" | grep -v 'migrations applied' > ~/fp-before.txt
wc -l < ~/fp-before.txt

echo "== 2. 止めて、DB をボリュームの外に写す"
docker compose stop backend
sleep 15   # Litestream が最後の変更を送り終えるのを待つ
docker compose stop litestream
COPY=~/machikoe/pre-restore-test-$(date +%Y%m%d-%H%M%S)
mkdir -p "$COPY"
docker compose run -T --rm --no-deps -v "$COPY:/b" restore 'cp -a /data/. /b/'
test -s "$COPY/civic.db" || { echo "写しが無いので止めます"; docker compose up -d; exit 1; }

echo "== 3. ボリュームを消して、起動（R2 から戻る）"
docker compose down
docker volume rm civic-platform_db
docker compose up -d
sleep 15
docker compose ps -a --format '{{.Service}} {{.Status}}'

echo "== 4. 比べる"
fp "$@" | grep -v 'migrations applied' > ~/fp-after.txt
wc -l < ~/fp-after.txt
if diff ~/fp-before.txt ~/fp-after.txt; then
  echo "一致しました。写しは確かめが済んだら消す: sudo rm -rf $COPY"
else
  echo "違いがあります。写しは $COPY"
  exit 1
fi
