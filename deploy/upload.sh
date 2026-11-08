#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
#
# 手元で作った資料（原本と抽出の JSON）を本番の backend のコンテナに送り、import で取り込む。
# 既定は検査だけ（import --dry-run）。--apply を付けると、検査のあとに下書きとして保存する。
# 送ったファイルは、終わったらコンテナから消す（原本は保持しない。docs/design/collection.md）。
# 手順の全体は docs/operations/deploy.md の「資料を本番に入れる」
#
# 使い方:
#   deploy/upload.sh [--apply] [--extraction <JSON>] <対象> <原本> [import の引数...]
#
#   <対象>      minutes（会議録）・budget（予算の概要）など、import が --file を受け取る対象
#   <原本>      会議録の UTF-8 のテキスト・予算の PDF など。隣の <名前>.extraction.json と <名前>.meta.json も送る
#   --extraction  抽出の JSON が隣に無い（名前が違う）ときに指定する。<名前>.extraction.json として送る
#   import の引数  -m <slug>・-y <年度>・--url <URL> など。そのまま import に渡す
#
# 送り先（環境変数）:
#   DEPLOY_SSH  ssh に渡す接続先（~/.ssh/config のホストの別名がおすすめ。例: machikoe）。
#               空なら、このスクリプトを本番のサーバーの上で動かしているものとして、ssh を使わない
#   DEPLOY_DIR  サーバーで compose.yaml と .env を置いたフォルダ（例: ~/machikoe/deploy）。
#               DEPLOY_SSH が空なら、既定はこのスクリプトのあるフォルダ
#
# 例:
#   export DEPLOY_SSH=machikoe DEPLOY_DIR='~/machikoe/deploy'
#   deploy/upload.sh minutes data/private/minutes/2026R08_0616_02-02.txt -m otawara
#   deploy/upload.sh --apply minutes data/private/minutes/2026R08_0616_02-02.txt -m otawara
#   deploy/upload.sh --extraction data/private/budget/r8-extraction.json \
#     budget data/private/budget/R8yosangaiyou.pdf -m otawara -y 2026 --url https://...

set -euo pipefail

usage() {
  sed -n '/^# 使い方:/,/^# 例:/p' "$0" | sed 's/^# \{0,1\}//' | sed '$d' >&2
  exit 2
}

apply=false
extraction=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --apply) apply=true; shift ;;
    --extraction) [[ $# -ge 2 ]] || usage; extraction="$2"; shift 2 ;;
    -h|--help) usage ;;
    --*) echo "知らないオプション: $1" >&2; usage ;;
    *) break ;;
  esac
done
[[ $# -ge 2 ]] || usage
target="$1"
original="$2"
shift 2
import_args=("$@")

case "$target" in
  *[!a-z_]*|"") echo "対象の名前が正しくありません: $target" >&2; exit 2 ;;
esac
[[ -f "$original" ]] || { echo "原本がありません: $original" >&2; exit 1; }
[[ -z "$extraction" || -f "$extraction" ]] || { echo "抽出の JSON がありません: $extraction" >&2; exit 1; }

# --- 送るファイルを1つのフォルダにそろえる（import が原本の隣を読む名前にする） ---
base="$(basename "$original")"
stem="${base%.*}"
dir="$(dirname "$original")"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
cp "$original" "$stage/$base"
if [[ -n "$extraction" ]]; then
  cp "$extraction" "$stage/$stem.extraction.json"
elif [[ -f "$dir/$stem.extraction.json" ]]; then
  cp "$dir/$stem.extraction.json" "$stage/"
else
  echo "抽出の JSON がありません（$dir/$stem.extraction.json）。--extraction で指定してください" >&2
  exit 1
fi
[[ -f "$dir/$stem.meta.json" ]] && cp "$dir/$stem.meta.json" "$stage/"

# --- サーバーでコマンドを動かす（DEPLOY_SSH が空ならこの場で） ---
if [[ -n "${DEPLOY_SSH:-}" ]]; then
  [[ -n "${DEPLOY_DIR:-}" ]] || { echo "DEPLOY_DIR を指定してください（サーバーの compose.yaml のフォルダ）" >&2; exit 2; }
  remote_dir="$DEPLOY_DIR"
else
  remote_dir="${DEPLOY_DIR:-$(cd "$(dirname "$0")" && pwd)}"
fi

# 引数を、サーバーのシェルにそのまま渡せる形にする
quote() { local out=() a; for a in "$@"; do out+=("$(printf '%q' "$a")"); done; echo "${out[*]}"; }

on_server() {
  # DEPLOY_DIR の ~ はサーバーの側で展開する
  local cmd="cd $remote_dir && $1"
  if [[ -n "${DEPLOY_SSH:-}" ]]; then
    # shellcheck disable=SC2086  # DEPLOY_SSH は ssh のオプションを含んでよい
    ssh $DEPLOY_SSH "$cmd"
  else
    bash -c "$cmd"
  fi
}

inbox="/data/private/upload/$target"
in_container="$inbox/$base"
cleanup_remote() {
  on_server "docker compose exec -T backend rm -rf $(quote "$inbox")" >/dev/null 2>&1 || true
}

echo "送ります: $(cd "$stage" && ls | tr '\n' ' ')→ backend:$inbox/"
# コンテナの中は backend と同じ利用者で書く（/data/private の持ち主を変えない）
tar -C "$stage" -cf - --owner=0 --group=0 . | on_server \
  "docker compose exec -T backend sh -c $(quote "mkdir -p '$inbox' && tar -C '$inbox' -xf - --no-same-owner")"
trap 'rm -rf "$stage"; cleanup_remote' EXIT

run_import() {
  on_server "docker compose exec -T backend backend import $(quote "$target" --file "$in_container" "${import_args[@]}" "$@")"
}

echo "=== 検査（import --dry-run）"
run_import --dry-run

if $apply; then
  echo "=== 保存（import）"
  run_import
  echo "下書きにしました。管理画面（/operator）か show / publish で確かめて公開してください"
else
  echo "検査だけです。保存するときは --apply を付けて、もう一度実行してください"
fi
