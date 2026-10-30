// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useState } from "react";

import { EmptyState } from "@/components/ui/empty-state";
import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorApi, operatorRecommendationApi } from "@/lib/api/paths";
import type {
  RecommendationCandidates,
  RecommendationDetail,
  RecommendationSummary,
  RecommendationWithText,
} from "@/lib/api/types/recommendations";
import { useApi } from "@/lib/api/use-api";
import { operatorHref } from "@/lib/operator";

import { useAction } from "./use-action";

const INPUT =
  "w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm dark:border-zinc-700 dark:bg-zinc-900";
const SMALL_BUTTON = `h-8 px-3 text-xs ${BUTTON_SECONDARY}`;
const MAX_TITLE = 60;
const MAX_PREFACE = 400;

/**
 * 提言（ADR 0005）: 締め切ったトピックを選んで下書きを作り、見出しと前書きを確かめて公開する。
 * 中身（合意点・意見が分かれた点）はプログラムが決めるので、ここでは足し引きしない。
 * 公開した提言からは、第2回のトピックの下書きを作れる
 */
export function Recommendations({ slug }: { slug: string }) {
  const [version, setVersion] = useState(0);
  const refresh = () => setVersion((v) => v + 1);
  const candidates = useApi<RecommendationCandidates>(
    `${operatorApi(slug)}/recommendation-candidates`,
    version,
  );
  const drafts = useApi<RecommendationSummary[]>(
    `${operatorRecommendationApi(slug)}?status=draft`,
    version,
  );

  return (
    <div className="flex flex-col gap-8">
      <section aria-label="下書き">
        <h2 className="mb-2 text-sm font-semibold">下書き</h2>
        {drafts.error && <p className={ERROR_TEXT}>読み込めませんでした</p>}
        {drafts.data && drafts.data.length === 0 && (
          <EmptyState>下書きの提言はありません</EmptyState>
        )}
        <ul className="flex flex-col gap-4">
          {drafts.data?.map((d) => (
            <li key={d.id}>
              <DraftRecommendation slug={slug} id={d.id} onDone={refresh} />
            </li>
          ))}
        </ul>
      </section>

      <section aria-label="新しい提言">
        <h2 className="mb-2 text-sm font-semibold">新しい提言を作る</h2>
        {candidates.error && <p className={ERROR_TEXT}>読み込めませんでした</p>}
        {candidates.data && (
          <NewRecommendation
            slug={slug}
            candidates={candidates.data}
            onCreated={refresh}
          />
        )}
      </section>

      {candidates.data && (
        <section aria-label="公開済みの提言">
          <h2 className="mb-2 text-sm font-semibold">公開済み</h2>
          {candidates.data.recommendations.length === 0 ? (
            <EmptyState>公開済みの提言はありません</EmptyState>
          ) : (
            <ul className="flex flex-col gap-3">
              {candidates.data.recommendations.map((r) => (
                <li key={r.id}>
                  <PublishedRecommendation
                    slug={slug}
                    summary={r}
                    onDone={refresh}
                  />
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </div>
  );
}

function NewRecommendation({
  slug,
  candidates,
  onCreated,
}: {
  slug: string;
  candidates: RecommendationCandidates;
  onCreated: () => void;
}) {
  const [topics, setTopics] = useState<Set<string>>(new Set());
  const [children, setChildren] = useState<Set<string>>(new Set());
  const [notice, setNotice] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const titles = new Map(candidates.topics.map((t) => [t.id, t.title]));

  function toggle(set: Set<string>, id: string) {
    const next = new Set(set);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return next;
  }

  async function create() {
    setNotice(null);
    const ok = await run(async () => {
      const r = await apiFetch<RecommendationWithText>(
        operatorRecommendationApi(slug),
        {
          method: "POST",
          body: { topic_ids: [...topics], child_ids: [...children] },
        },
      );
      setNotice(
        r.text_error ??
          (r.title
            ? "下書きを作りました。見出しと前書きは AI の下書きです。確かめてください"
            : "下書きを作りました。見出しと前書きを書いてください"),
      );
    }, "作れませんでした");
    if (ok) {
      setTopics(new Set());
      setChildren(new Set());
      onCreated();
    }
  }

  if (
    candidates.topics.length === 0 &&
    candidates.recommendations.length === 0
  ) {
    return (
      <EmptyState>
        提言にできるトピックはまだありません（締め切っていて、分析の対象が5人以上、意見のグループが2つ以上のもの）
      </EmptyState>
    );
  }
  return (
    <div className="flex flex-col gap-3">
      {candidates.groups.length > 0 && (
        <div>
          <p className="text-xs text-zinc-500">
            まとまりの候補（同じ分野で似ているもの。まとめるかはあなたが決めます）
          </p>
          <ul className="mt-1 flex flex-col gap-1">
            {candidates.groups.map((g) => (
              <li key={g.join(",")}>
                <button
                  type="button"
                  className="text-left text-sm underline underline-offset-2"
                  onClick={() => setTopics(new Set(g))}
                >
                  {g.map((id) => titles.get(id) ?? id).join(" ＋ ")}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
      {candidates.topics.length > 0 && (
        <fieldset>
          <legend className="text-xs text-zinc-500">締め切ったトピック</legend>
          <ul className="mt-1 flex flex-col gap-1">
            {candidates.topics.map((t) => (
              <li key={t.id}>
                <label className="flex items-start gap-2 text-sm">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={topics.has(t.id)}
                    onChange={() => setTopics((s) => toggle(s, t.id))}
                  />
                  <span>
                    {t.title}
                    <span className="ml-1 text-xs text-zinc-500">
                      {t.participants}人
                      {t.fields.length > 0 && ` ・ ${t.fields.join("・")}`}
                    </span>
                  </span>
                </label>
              </li>
            ))}
          </ul>
        </fieldset>
      )}
      {candidates.recommendations.length > 0 && (
        <fieldset>
          <legend className="text-xs text-zinc-500">
            まとめる提言（子の提言の見出し・前書きは使わず、項目の原文と数字をたどって集めます）
          </legend>
          <ul className="mt-1 flex flex-col gap-1">
            {candidates.recommendations.map((r) => (
              <li key={r.id}>
                <label className="flex items-start gap-2 text-sm">
                  <input
                    type="checkbox"
                    className="mt-1"
                    checked={children.has(r.id)}
                    onChange={() => setChildren((s) => toggle(s, r.id))}
                  />
                  <span>{r.title}</span>
                </label>
              </li>
            ))}
          </ul>
        </fieldset>
      )}
      <div>
        <button
          type="button"
          disabled={busy || (topics.size === 0 && children.size === 0)}
          onClick={create}
          className={`h-10 px-5 text-sm ${BUTTON_PRIMARY}`}
        >
          下書きを作る
        </button>
      </div>
      {notice && <p className="text-sm text-zinc-600">{notice}</p>}
      {error && <p className={ERROR_TEXT}>{error}</p>}
    </div>
  );
}

function DraftRecommendation({
  slug,
  id,
  onDone,
}: {
  slug: string;
  id: string;
  onDone: () => void;
}) {
  const api = operatorRecommendationApi(slug, id);
  const detail = useApi<RecommendationDetail>(api);
  const r = detail.data;
  if (!r) {
    return detail.error ? (
      <p className={ERROR_TEXT}>読み込めませんでした</p>
    ) : (
      <p className="text-sm text-zinc-500">読み込み中…</p>
    );
  }
  return (
    <DraftEditor
      key={`${r.title}\n${r.preface}`}
      api={api}
      recommendation={r}
      onChange={(next) => detail.update(() => next)}
      onDone={onDone}
    />
  );
}

function DraftEditor({
  api,
  recommendation: r,
  onChange,
  onDone,
}: {
  api: string;
  recommendation: RecommendationDetail;
  onChange: (r: RecommendationDetail) => void;
  onDone: () => void;
}) {
  const [title, setTitle] = useState(r.title);
  const [preface, setPreface] = useState(r.preface);
  const [notice, setNotice] = useState<string | null>(null);
  const { busy, error, run } = useAction();
  const dirty = title !== r.title || preface !== r.preface;
  const consensus = r.items.filter((i) => i.kind === "consensus");
  const divided = r.items.filter((i) => i.kind === "divided");

  async function save() {
    return run(
      async () =>
        onChange(
          await apiFetch<RecommendationDetail>(api, {
            method: "PATCH",
            body: { title, preface },
          }),
        ),
      "保存できませんでした",
    );
  }

  async function rewrite() {
    setNotice(null);
    await run(async () => {
      const next = await apiFetch<RecommendationWithText>(`${api}/text`, {
        method: "POST",
      });
      setNotice(next.text_error);
      onChange(next);
    }, "AI に頼めませんでした");
  }

  async function publish() {
    if (dirty && !(await save())) return;
    if (
      !window.confirm(
        `「${title}」を公開します。公開したあとは直せません。よろしいですか`,
      )
    )
      return;
    const ok = await run(
      () => apiFetch(`${api}/publish`, { method: "POST" }),
      "公開できませんでした",
    );
    if (ok) onDone();
  }

  async function remove() {
    if (!window.confirm("この下書きを消します。よろしいですか")) return;
    const ok = await run(
      () => apiFetch(api, { method: "DELETE" }),
      "消せませんでした",
    );
    if (ok) onDone();
  }

  return (
    <article className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-xs text-zinc-500">
        元のトピック: {r.topics.map((t) => t.title).join("、") || "なし"}
        {r.children.length > 0 &&
          ` ・ まとめる提言: ${r.children.map((c) => c.title).join("、")}`}
      </p>
      <label className="mt-3 block text-xs text-zinc-600 dark:text-zinc-400">
        見出し（{MAX_TITLE}文字まで。中立に）
        <input
          className={`mt-1 ${INPUT}`}
          value={title}
          maxLength={MAX_TITLE}
          onChange={(e) => setTitle(e.target.value)}
        />
      </label>
      <label className="mt-2 block text-xs text-zinc-600 dark:text-zinc-400">
        前書き（{MAX_PREFACE}文字まで。結論や評価を書かない）
        <textarea
          className={`mt-1 ${INPUT}`}
          rows={4}
          value={preface}
          maxLength={MAX_PREFACE}
          onChange={(e) => setPreface(e.target.value)}
        />
      </label>
      <div className="mt-2 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy || !dirty}
          onClick={save}
          className={SMALL_BUTTON}
        >
          保存
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={rewrite}
          className={SMALL_BUTTON}
        >
          AI に書き直してもらう
        </button>
      </div>

      <details className="mt-3">
        <summary className="cursor-pointer text-sm">
          中身を見る（合意点 {consensus.length} 件 ・ 分かれた点{" "}
          {divided.length} 件）
        </summary>
        <ul className="mt-2 flex flex-col gap-1 text-sm">
          {r.items.map((i) => (
            <li key={i.statement_id}>
              <span className="mr-1 text-xs text-zinc-500">
                {i.kind === "consensus" ? "合意点" : "分かれた点"}
              </span>
              {i.body}
            </li>
          ))}
        </ul>
      </details>

      <div className="mt-4 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy || title.trim() === "" || r.items.length === 0}
          onClick={publish}
          className={`h-9 px-4 text-sm ${BUTTON_PRIMARY}`}
        >
          公開する
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={remove}
          className={`h-9 px-4 text-sm ${BUTTON_SECONDARY}`}
        >
          下書きを消す
        </button>
      </div>
      {r.items.length === 0 && (
        <p className="mt-2 text-xs text-zinc-500">
          合意点も意見が分かれた点も無いので、公開できません
        </p>
      )}
      {notice && <p className="mt-2 text-sm text-zinc-600">{notice}</p>}
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
    </article>
  );
}

function PublishedRecommendation({
  slug,
  summary,
  onDone,
}: {
  slug: string;
  summary: RecommendationSummary;
  onDone: () => void;
}) {
  const api = operatorRecommendationApi(slug, summary.id);
  const [created, setCreated] = useState(false);
  const { busy, error, run } = useAction();

  async function secondRound() {
    if (
      !window.confirm(
        "この提言を最初の意見にした、第2回のトピックの下書きを作ります。よろしいですか",
      )
    )
      return;
    const ok = await run(
      () => apiFetch(`${api}/second-round`, { method: "POST" }),
      "作れませんでした",
    );
    if (ok) setCreated(true);
  }

  async function archive() {
    if (
      !window.confirm(
        `「${summary.title}」を取り下げます（公開をやめます）。よろしいですか`,
      )
    )
      return;
    const ok = await run(
      () => apiFetch(`${api}/archive`, { method: "POST" }),
      "取り下げられませんでした",
    );
    if (ok) onDone();
  }

  return (
    <article className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <Link
        href={`/${slug}/recommendations/${summary.id}`}
        className="font-medium underline underline-offset-2"
      >
        {summary.title}
      </Link>
      <p className="mt-1 text-xs text-zinc-500">
        合意点 {summary.consensus} 件 ・ 分かれた点 {summary.divided} 件
      </p>
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={secondRound}
          className={SMALL_BUTTON}
        >
          第2回のトピックを作る
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={archive}
          className={SMALL_BUTTON}
        >
          取り下げる
        </button>
      </div>
      {created && (
        <p className="mt-2 text-sm text-zinc-600">
          第2回のトピックの下書きを作りました。
          <Link
            href={operatorHref(slug, "drafts")}
            className="underline underline-offset-2"
          >
            下書き
          </Link>
          で最初の意見を確かめ、多ければ減らしてから公開してください。
        </p>
      )}
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
    </article>
  );
}
