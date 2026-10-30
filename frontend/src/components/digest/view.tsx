// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useEffect, useRef, useState } from "react";

import { BUTTON_PRIMARY, BUTTON_SECONDARY } from "@/components/ui/styles";
import type { Digest, TopicDigest } from "@/lib/api/types/digest";
import { proposeFromDigestHref, ratioLabel } from "@/lib/digest";
import { RESIDENCY_ORDER, RESIDENCY_SHORT_LABELS } from "@/lib/labels";

import { DigestGroups, DigestItems, type UseToggle } from "./items";
import { PrintButton } from "./print-button";
import { STICKY_BLOCK, STICKY_TOPIC } from "./sticky";

/** 提案の最初の意見の上限（backend の progress::PROPOSAL_SEEDS） */
const MAX_USE = 10;

/**
 * 市民のまとめ: 選んだトピックごとに、合意点・意見が分かれた点・グループの代表的な意見を並べる。
 * 「使う」で選んだ意見を最初の意見にして、自分で新しいトピックを提案できる（AI でまとめない。ADR 0005）
 */
export function DigestView({
  slug,
  digest,
  printHref,
}: {
  slug: string;
  digest: Digest;
  /** レイヤで開いたとき: 印刷はできないので、普通のページを開くリンクにする（レイヤは印刷に出ない） */
  printHref?: string;
}) {
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const use: UseToggle = {
    selected,
    onToggle: (id) =>
      setSelected((s) => {
        const next = new Set(s);
        if (next.has(id)) next.delete(id);
        else if (next.size < MAX_USE) next.add(id);
        return next;
      }),
  };
  const others = digest.topics.slice(1).map((t) => t.id);

  return (
    <>
      <div className="flex flex-col gap-3">
        <p className="text-sm leading-6 text-zinc-700 dark:text-zinc-300">
          締め切った{digest.topics.length}
          件のトピックの投票から、どの意見グループでも賛成が多かった意見（合意点）と、グループによって賛否が分かれた意見を、原文と数字のまま並べています。これを見ながら、次に話し合いたいトピックを提案できます。
        </p>
        <div>
          {printHref ? (
            // ページの読み込みからやり直して（インターセプトしない）、普通のまとめのページを開く
            <a
              href={printHref}
              className={`inline-flex h-9 items-center px-4 text-sm ${BUTTON_SECONDARY}`}
            >
              印刷・PDF 用に開く
            </a>
          ) : (
            <PrintButton />
          )}
        </div>
      </div>

      {digest.topics.map((t) => (
        <TopicSection
          key={t.id}
          topic={t}
          slug={slug}
          dividedThreshold={digest.divided_threshold}
          use={use}
        />
      ))}

      <p className="text-xs leading-5 text-zinc-500">
        グループ
        A・B…は、トピックごとの投票の傾向で分けた意見のグループです（年齢や住む地区などで分けたものではありません）。トピックが違えば、同じ記号でも別のグループです。ここに並ぶのはすべて投票された意見の原文で、AI
        も運営者も書き足していません。
      </p>

      <div className="sticky bottom-0 z-30 -mx-4 border-t border-zinc-200 bg-white/95 px-4 py-3 backdrop-blur print:hidden dark:border-zinc-800 dark:bg-zinc-950/95">
        <Link
          href={proposeFromDigestHref(slug, digest.anchor, others, [
            ...selected,
          ])}
          className={`flex h-11 items-center justify-center text-sm ${BUTTON_PRIMARY}`}
        >
          このまとめから新しいトピックを提案する
          {selected.size > 0 && `（意見 ${selected.size} 件を使う）`}
        </Link>
        <p className="mt-1 text-center text-xs text-zinc-500">
          「使う」を押した意見が、最初の意見に入ります（{MAX_USE}
          件まで）。提案は1週間に1件までです
        </p>
      </div>
    </>
  );
}

function TopicSection({
  topic: t,
  slug,
  dividedThreshold,
  use,
}: {
  topic: TopicDigest;
  slug: string;
  dividedThreshold: number;
  use: UseToggle;
}) {
  const head = useRef<HTMLDivElement>(null);
  const [headHeight, setHeadHeight] = useState(0);
  const [open, setOpen] = useState(false);
  // 1段目の高さを測り、2段目をその下に貼り付ける
  useEffect(() => {
    const el = head.current;
    if (!el) return;
    // 小数まで測る（offsetHeight は整数に丸めるので、2段目との間に1px未満の隙間ができる）
    const observer = new ResizeObserver(() =>
      setHeadHeight(el.getBoundingClientRect().height),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <section
      className="flex flex-col gap-4 border-t border-zinc-200 dark:border-zinc-800"
      // 2段目は1段目の下端に1pxだけ重ねる（1段目は枠の上端より1px上にあるので、高さから2px引く。
      // 端数で隙間ができ、下を流れる中身が見えないように）
      style={
        {
          "--topic-head": `${Math.max(0, headHeight - 2)}px`,
        } as React.CSSProperties
      }
    >
      {/* トピックの題と説明は、このトピックの節を見ている間は上に残す（説明は2行まで。押すと全部） */}
      <div ref={head} className={STICKY_TOPIC}>
        <h2 className="truncate text-base font-bold">
          <Link
            href={`/${slug}/topics/${t.id}`}
            className="underline-offset-2 hover:underline"
          >
            {t.title}
          </Link>
        </h2>
        {t.description && (
          <button
            type="button"
            aria-expanded={open}
            onClick={() => setOpen((o) => !o)}
            className={`mt-0.5 block text-left text-xs leading-5 text-zinc-600 dark:text-zinc-400 ${open ? "" : "line-clamp-2"}`}
          >
            {t.description}
          </button>
        )}
      </div>
      <p className="-mt-2 text-xs leading-5 text-zinc-500">
        分析の対象 {t.analyzed} 人
        {t.sufficient &&
          `（${RESIDENCY_ORDER.map((key) => {
            const n = t.residency[key];
            return `${RESIDENCY_SHORT_LABELS[key]} ${n == null ? "—" : `${n}人`}`;
          }).join("・")}）`}
      </p>
      {!t.sufficient ? (
        <p className="text-sm text-zinc-500">
          分析の対象が5人に満たないので、合意点などは出していません。
        </p>
      ) : (
        <>
          <Block
            title={`合意点（${t.consensus.length}件）`}
            note="どの意見グループでも、8割以上が賛成した意見"
          >
            <DigestItems items={t.consensus} use={use} />
          </Block>
          <Block
            title={`意見が分かれた点（${t.divided.length}件）`}
            note={`あるグループでは${ratioLabel(dividedThreshold)}以上が賛成し、別のグループでは${ratioLabel(dividedThreshold)}以上が反対した意見`}
          >
            <DigestItems items={t.divided} use={use} />
          </Block>
          <Block
            title="グループごとの代表的な意見"
            note="そのグループが、ほかのグループと比べて特に賛成・反対した意見"
          >
            <DigestGroups groups={t.groups} use={use} />
          </Block>
        </>
      )}
    </section>
  );
}

function Block({
  title,
  note,
  children,
}: {
  title: string;
  note: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      {/* 節の見出しは、節の中を見ている間はトピックの題の下に残す */}
      <div className={STICKY_BLOCK}>
        <h3 className="text-sm font-semibold">{title}</h3>
        <p className="text-xs text-zinc-500">{note}</p>
      </div>
      <div className="mt-2">{children}</div>
    </div>
  );
}
