// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useCallback, useEffect, useRef, useState } from "react";

import {
  COLORED_FIELDS,
  FLAG_LABEL,
  type FieldGroup,
  OTHER_FIELDS,
  OTHER_FIELD_COLOR as OTHER_COLOR,
} from "@/lib/finances/budget-fields";
import { roughYen, shareText, thousandYen } from "@/lib/finances/money";
import { type Rect, squarify } from "@/lib/finances/treemap";
import type { BudgetProject } from "@/lib/api/types/finances";
import { useDismiss } from "@/lib/use-dismiss";

/** 分野を開いたとき、事業の四角の数の上限と、四角にする最小の割合（それより小さい事業は「ほか」） */
const MAX_TILES = 20;
const MIN_SHARE = 0.015;
const HEIGHT = 260;
/** 名前を四角の中に出す大きさ（px） */
const LABEL_MIN_W = 72;
const LABEL_MIN_H = 30;
const GAP = 2;
const UNLISTED_COLOR = "var(--treemap-unlisted)";

type Tile = {
  key: string;
  label: string;
  amount: number;
  color: string;
  /** 吹き出しの中身 */
  detail: React.ReactNode;
  action?: React.ReactNode;
};

/**
 * 主な事業のツリーマップ（2段）。1段目は分野、分野を開くとその分野の事業。
 * 四角の名前は、大きい四角だけ中に出し、ほかはタップで吹き出しに出す（InfoTip と同じふるまい）。
 * 色だけで分野を見分けさせないため、一覧の分野のチップにも同じ色の印を付ける（凡例の代わり）
 */
export function BudgetTreemap({
  projects,
  groups,
  total,
  population,
  pdfUrl,
  field,
  onField,
}: {
  projects: BudgetProject[];
  groups: FieldGroup[];
  /** 予算の全体（千円） */
  total: number;
  population: number | null;
  pdfUrl: string | null;
  /** 開いている分野（null は1段目、OTHER_FIELDS は「ほかの分野」） */
  field: string | null;
  onField: (id: string | null) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [active, setActive] = useState<string | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(([entry]) =>
      setWidth(entry.contentRect.width),
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  // 吹き出しは1つだけ。画面のどこかに触れると閉じる
  const close = useCallback(() => setActive(null), []);
  const announce = useDismiss(active !== null, close, ref);

  const perCapita = (amount: number) =>
    population ? (
      <span className="block">
        1人あたり {roughYen((amount * 1000) / population)}
      </span>
    ) : null;

  const open = (next: string | null) => {
    setActive(null);
    onField(next);
  };

  const colored = groups.slice(0, COLORED_FIELDS);
  const rest = groups.slice(COLORED_FIELDS);
  const current = groups.find((g) => g.id === field);
  let tiles: Tile[];
  let heading: React.ReactNode = null;

  if (field === null || field === OTHER_FIELDS) {
    const listed = projects.reduce((a, p) => a + p.amount, 0);
    const shown = field === null ? colored : rest;
    const base =
      field === null ? total : rest.reduce((a, g) => a + g.amount, 0);
    tiles = shown.map((g) => ({
      key: g.id,
      label: g.name,
      amount: g.amount,
      color: field === null ? g.color : OTHER_COLOR,
      detail: (
        <>
          <span className="block font-semibold">{g.name}</span>
          <span className="block tabular-nums">
            {thousandYen(g.amount)}（{field === null ? "予算の" : ""}
            {shareText(g.amount / base)}）
          </span>
          {perCapita(g.amount)}
          <span className="block text-zinc-400 dark:text-zinc-500">
            事業 {g.count}件
          </span>
        </>
      ),
      action: (
        <button
          type="button"
          onClick={() => open(g.id)}
          className="mt-1.5 font-semibold underline underline-offset-2"
        >
          事業を見る ›
        </button>
      ),
    }));
    if (field === null && rest.length > 0) {
      const amount = rest.reduce((a, g) => a + g.amount, 0);
      tiles.push({
        key: OTHER_FIELDS,
        label: "ほかの分野",
        amount,
        color: OTHER_COLOR,
        detail: (
          <>
            <span className="block font-semibold">
              ほかの分野（{rest.length}）
            </span>
            <span className="block tabular-nums">
              {thousandYen(amount)}（予算の{shareText(amount / total)}）
            </span>
            <span className="block text-zinc-400 dark:text-zinc-500">
              {rest.map((g) => g.name).join("・")}
            </span>
          </>
        ),
        action: (
          <button
            type="button"
            onClick={() => open(OTHER_FIELDS)}
            className="mt-1.5 font-semibold underline underline-offset-2"
          >
            分野を見る ›
          </button>
        ),
      });
    }
    if (field === null && total > listed) {
      const amount = total - listed;
      tiles.push({
        key: "__unlisted",
        label: "概要に載らない分",
        amount,
        color: UNLISTED_COLOR,
        detail: (
          <>
            <span className="block font-semibold">
              概要に事業として載らない分
            </span>
            <span className="block tabular-nums">
              {thousandYen(amount)}（予算の{shareText(amount / total)}）
            </span>
            <span className="block text-zinc-400 dark:text-zinc-500">
              予算の全体から、概要に挙がった主な事業の合計を引いた額です
            </span>
          </>
        ),
      });
    }
    if (field === OTHER_FIELDS) {
      heading = <>ほかの分野 · {thousandYen(base)}</>;
    }
  } else {
    const items = projects
      .filter((p) => p.tags.some((t) => t.id === field))
      .sort((a, b) => b.amount - a.amount);
    const sum = items.reduce((a, p) => a + p.amount, 0);
    const big = items
      .filter((p) => p.amount / sum >= MIN_SHARE)
      .slice(0, MAX_TILES);
    const small = items.slice(big.length);
    const color = current?.color ?? OTHER_COLOR;
    tiles = big.map((p) => ({
      key: `${p.name}-${p.page}`,
      label: p.name,
      amount: p.amount,
      color,
      detail: (
        <>
          <span className="block font-semibold">
            {p.flag && (
              <span className="mr-1 font-normal text-amber-300 dark:text-amber-700">
                {FLAG_LABEL[p.flag]}
              </span>
            )}
            {p.name}
          </span>
          <span className="block tabular-nums">
            {thousandYen(p.amount)}（この分野の{shareText(p.amount / sum)}）
          </span>
          {p.per_capita !== null && (
            <span className="block">1人あたり {roughYen(p.per_capita)}</span>
          )}
          {p.tags.length > 1 && (
            <span className="block text-zinc-400 dark:text-zinc-500">
              {p.tags.map((t) => `#${t.name}`).join(" ")}
            </span>
          )}
        </>
      ),
      action: pdfUrl ? (
        <a
          href={`${pdfUrl}#page=${p.page}`}
          target="_blank"
          rel="noopener noreferrer"
          className="mt-1.5 inline-block font-semibold underline underline-offset-2"
        >
          説明は原本 p.{p.page}
        </a>
      ) : undefined,
    }));
    if (small.length > 0) {
      const amount = small.reduce((a, p) => a + p.amount, 0);
      tiles.push({
        key: "__small",
        label: `ほか${small.length}件`,
        amount,
        color: OTHER_COLOR,
        detail: (
          <>
            <span className="block font-semibold">
              小さい事業 {small.length}件
            </span>
            <span className="block tabular-nums">
              {thousandYen(amount)}（この分野の{shareText(amount / sum)}）
            </span>
            <span className="block text-zinc-400 dark:text-zinc-500">
              下の一覧で見られます
            </span>
          </>
        ),
      });
    }
    heading = (
      <>
        {current?.name} · {thousandYen(sum)} · {items.length}件
      </>
    );
  }

  const rects: Rect[] = squarify(
    tiles.map((t) => t.amount),
    width,
    HEIGHT,
  );
  const activeIndex = tiles.findIndex((t) => t.key === active);
  // 「ほかの分野」から開いた分野は、「ほかの分野」に戻る
  const backTo =
    field !== OTHER_FIELDS && rest.some((g) => g.id === field)
      ? OTHER_FIELDS
      : null;

  return (
    <div>
      {field !== null && (
        <div className="mb-2 flex items-center gap-2 text-sm">
          <button
            type="button"
            onClick={() => open(backTo)}
            className="-ml-1 inline-flex min-h-9 shrink-0 items-center rounded-full px-2 text-zinc-600 dark:text-zinc-400"
          >
            ‹ {backTo === null ? "すべての分野" : "ほかの分野"}
          </button>
          <span className="truncate font-semibold tabular-nums">{heading}</span>
        </div>
      )}
      <div
        ref={ref}
        className="relative w-full"
        style={{ height: HEIGHT }}
        data-swipe-ignore
      >
        {width > 0 &&
          tiles.map((t, i) => {
            const r = rects[i];
            if (r.w <= 0 || r.h <= 0) return null;
            const showLabel = r.w >= LABEL_MIN_W && r.h >= LABEL_MIN_H;
            return (
              <button
                key={t.key}
                type="button"
                aria-label={`${t.label} ${thousandYen(t.amount)}`}
                aria-expanded={active === t.key}
                onClick={() => {
                  if (active !== t.key) announce();
                  setActive(active === t.key ? null : t.key);
                }}
                className="absolute overflow-hidden rounded-[4px] p-1 text-left transition-opacity"
                style={{
                  left: r.x + GAP / 2,
                  top: r.y + GAP / 2,
                  width: Math.max(r.w - GAP, 1),
                  height: Math.max(r.h - GAP, 1),
                  background: t.color,
                  opacity: active && active !== t.key ? 0.45 : 1,
                }}
              >
                {showLabel && (
                  <span className="block max-w-full truncate rounded bg-white/90 px-1 text-[11px] leading-4 text-zinc-900 dark:bg-zinc-950/85 dark:text-zinc-100">
                    {t.label}
                  </span>
                )}
              </button>
            );
          })}
        {activeIndex >= 0 && (
          <Bubble rect={rects[activeIndex]} width={width}>
            {tiles[activeIndex].detail}
            {tiles[activeIndex].action}
          </Bubble>
        )}
      </div>
      <p className="mt-2 text-xs text-zinc-500">
        タップでくわしく。面積が金額の大きさです
      </p>
    </div>
  );
}

/** 四角の下（入らなければ上）に出す吹き出し。左右は枠からはみ出さないようにずらす */
function Bubble({
  rect,
  width,
  children,
}: {
  rect: Rect;
  width: number;
  children: React.ReactNode;
}) {
  const bubbleWidth = Math.min(240, width);
  const left = Math.min(Math.max(rect.x, 0), width - bubbleWidth);
  const below = rect.y + rect.h < HEIGHT * 0.6;
  return (
    <div
      role="tooltip"
      className="absolute z-30 rounded-xl bg-zinc-900 px-3 py-2 text-xs leading-5 text-white shadow-lg dark:bg-zinc-100 dark:text-zinc-900"
      style={{
        left,
        width: bubbleWidth,
        ...(below
          ? { top: rect.y + rect.h + 4 }
          : { bottom: HEIGHT - rect.y + 4 }),
      }}
    >
      {children}
    </div>
  );
}
