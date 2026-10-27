-- SPDX-License-Identifier: AGPL-3.0-only
-- 収集の条件付き取得の記録（docs/design/collection.md「取得の共通処理」）。
-- 中身は持たない。次の取得で ETag / Last-Modified を送り、変わっていなければ処理を飛ばす。
-- 決算カードの Excel は都道府県ごとに1つで、同じ県の自治体が同じ URL を使うため、収集先ごとに記録する
CREATE TABLE fetch_cache (
    source_id     TEXT NOT NULL REFERENCES collector_sources (id),
    url           TEXT NOT NULL,
    etag          TEXT,
    last_modified TEXT,
    -- 中身のハッシュ。ETag の無いサイトでも、変化が無いことが分かる
    sha256        TEXT NOT NULL,
    fetched_at    TEXT NOT NULL,
    PRIMARY KEY (source_id, url)
);
