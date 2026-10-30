// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { notFound } from "next/navigation";
import { Suspense } from "react";

import { OperatorLink } from "@/components/auth/operator-link";
import { MapTagList, TopicMapView } from "@/components/digest/map";
import { ConsensusToggle } from "@/components/topics/consensus-list";
import { municipalityApi, topicMapApi } from "@/lib/api/paths";
import { apiGet, requestCookie } from "@/lib/api/server";
import type { MunicipalityConsensus } from "@/lib/api/types/analysis";
import type { TopicMap } from "@/lib/api/types/digest";
import type { MunicipalityDetail } from "@/lib/api/types/municipalities";
import { MAP_ANCHOR } from "@/lib/digest";

/**
 * みんなの合意点（ADR 0005「市民のまとめ」）: 上に合意点の一覧、下に「これまでの議論」（締め切ったトピックの地図 → まとめ → 提案）。
 * 合意点を押すと、元のトピックを起点にした地図を開く（?focus=）。地図の分野は ?tag=
 */
export default function ConsensusPage({
  params,
  searchParams,
}: PageProps<"/[municipality]/consensus">) {
  // 戻るリンクと見出しを固定のヘッダーにし、その下をスクロールする（スクロールはページだけ）
  return (
    <main className="flex min-h-0 flex-1 flex-col">
      <Suspense fallback={<Skeleton />}>
        <ConsensusView params={params} searchParams={searchParams} />
      </Suspense>
    </main>
  );
}

async function ConsensusView({
  params,
  searchParams,
}: Pick<PageProps<"/[municipality]/consensus">, "params" | "searchParams">) {
  const { municipality: slug } = await params;
  const { tag, focus } = await searchParams;
  const path = municipalityApi(slug);
  // 受付中のトピックの合意点は、本人が投票し終えたものだけ（参加者ごとに違うので Cookie を渡す）。
  // 締め切ったトピックの合意点は誰にでも出す
  const cookie = await requestCookie();
  const focusId = typeof focus === "string" ? focus : null;
  const mapQuery =
    typeof tag === "string"
      ? topicMapApi(slug, tag)
      : focusId
        ? `${topicMapApi(slug)}?focus=${encodeURIComponent(focusId)}`
        : topicMapApi(slug);
  const [municipality, consensus, firstMap] = await Promise.all([
    apiGet<MunicipalityDetail>(path),
    apiGet<MunicipalityConsensus[]>(`${path}/consensus?include_closed=true`, {
      cookie,
    }),
    apiGet<TopicMap>(mapQuery),
  ]);
  if (!municipality || !consensus || !firstMap) {
    notFound();
  }
  // 地図はタグで絞ったものだけ。タグが決まらなければ（最初に開いたとき）、タグの一覧を出す
  const map = firstMap;

  return (
    <>
      <header className="shrink-0 border-b border-zinc-200 dark:border-zinc-800">
        <div className="mx-auto w-full max-w-md px-4 pb-3">
          <div className="flex items-center justify-between gap-4">
            <Link
              href={`/${municipality.slug}`}
              className="inline-flex h-10 items-center text-sm text-zinc-600 dark:text-zinc-400"
            >
              ← {municipality.name}のトピック
            </Link>
            <OperatorLink />
          </div>
          <h1 className="text-lg leading-snug font-bold">みんなの合意点</h1>
        </div>
      </header>

      {/* スクロールの位置の自動の補正（scroll anchoring）を切る。地図が縮むとその下のカードが上に動くので、
          補正が働くとスクロールの位置が戻って地図がまた広がり、揺れ続けるため */}
      <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain [overflow-anchor:none]">
        <div className="mx-auto flex w-full max-w-md flex-col gap-6 px-4 pt-4 pb-6">
          <ConsensusToggle slug={municipality.slug} items={consensus} />

          <section
            id={MAP_ANCHOR}
            aria-label="これまでの議論"
            className="scroll-mt-2"
          >
            {map.tag ? (
              // タグか起点が変わったら、選んでいたものを捨てる（key）
              <TopicMapView
                key={`${map.tag}:${focusId ?? ""}`}
                slug={municipality.slug}
                map={map}
                initialAnchor={focusId}
              />
            ) : (
              <>
                <h2 className="text-base font-bold">これまでの議論</h2>
                <p className="mb-3 text-xs leading-5 text-zinc-500">
                  締め切ったトピックの地図です。分野を選ぶと、似ているトピックほど近くに並んだ地図が開きます。似ているトピックを選んで、合意点や意見の分かれた点をまとめて見られ、そこから次のトピックを提案できます。
                </p>
                <MapTagList slug={municipality.slug} tags={map.tags} />
              </>
            )}
          </section>
        </div>
      </div>
    </>
  );
}

function Skeleton() {
  return (
    <div
      aria-hidden
      className="mx-auto w-full max-w-md animate-pulse px-4 py-3"
    >
      <div className="mb-4 h-4 w-40 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-6 w-1/2 rounded bg-zinc-200 dark:bg-zinc-800" />
      <div className="mb-3 h-24 rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
      <div className="aspect-square rounded-2xl bg-zinc-100 dark:bg-zinc-900" />
    </div>
  );
}
