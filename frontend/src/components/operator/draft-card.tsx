// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { chipClass } from "@/components/ui/chip";
import {
  BADGE_AMBER,
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorStatementApi, operatorTopicApi } from "@/lib/api/paths";
import type {
  OperatorStatement,
  OperatorTopic,
  TagChoice,
} from "@/lib/api/types/operator";
import { SOURCE_LABELS } from "@/lib/labels";
import { STATEMENT_STATUS_LABELS, formatDateTime } from "@/lib/operator";
import { MAX_STATEMENT_CHARS, countChars } from "@/lib/statement";

import { STICKY_CARD_HEAD } from "./sticky";
import { useAction } from "./use-action";

const INPUT =
  "w-full rounded-lg border border-zinc-300 bg-white px-3 py-2 text-sm dark:border-zinc-700 dark:bg-zinc-900";
const SMALL_BUTTON = `h-8 px-3 text-xs ${BUTTON_SECONDARY}`;

/**
 * 下書きのトピック1件: 出典と照らして、タイトル・説明・最初の意見・タグを直し、公開するか取り下げる。
 * 操作のあとは backend が返した最新のトピックで置き換える（`onChange`）
 */
export function DraftCard({
  slug,
  topic,
  tagChoices,
  onChange,
}: {
  slug: string;
  topic: OperatorTopic;
  tagChoices: TagChoice[];
  onChange: (topic: OperatorTopic) => void;
}) {
  const api = operatorTopicApi(slug, topic.id);
  const { busy, error, run } = useAction();

  /** 意見を直したあとなど、本文の無い応答のあとにトピックを読み直す */
  async function reload() {
    onChange(await apiFetch<OperatorTopic>(api));
  }

  async function enqueue() {
    await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(`${api}/queue`, { method: "POST" }),
        ),
      "公開待ちに入れられませんでした",
    );
  }

  async function setStatus(action: "publish" | "archive") {
    const message =
      action === "publish"
        ? "公開します。公開から90日の期限が付きます"
        : "取り下げます（公開しません）";
    if (!window.confirm(`「${topic.title}」を${message}。よろしいですか`))
      return;
    await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(`${api}/${action}`, { method: "POST" }),
        ),
      "変えられませんでした",
    );
  }

  return (
    <article className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-xs text-zinc-500">
        {topic.source_type === "citizen" && (
          <span className={`mr-2 rounded px-1.5 py-0.5 ${BADGE_AMBER}`}>
            {SOURCE_LABELS.citizen}
          </span>
        )}
        作成 {formatDateTime(topic.created_at)}
      </p>
      {topic.source_type === "citizen" && (
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          市民からの提案です。説明が一方の立場に寄っていないか、提案した人と違う立場の意見がわざと弱く書かれていないか、すでにある論点と同じでないかを確かめてください。取り下げると、提案した人には「見送り」と表示されます。
        </p>
      )}
      {topic.parents.length > 0 && (
        <p className="mt-2 text-xs leading-5 text-zinc-500">
          市民のまとめから: {topic.parents.map((p) => p.title).join("、")}
          （元のトピックの合意点・分かれた点を、一方の立場に寄せて書き換えていないかも確かめてください）
        </p>
      )}
      <TitleEditor slug={slug} topic={topic} onChange={onChange} />
      <SimilarTopics topic={topic} />

      {topic.sources.length > 0 && (
        <section aria-label="出典" className="mt-4">
          <h3 className="text-xs font-semibold text-zinc-600 dark:text-zinc-400">
            出典
          </h3>
          <ul className="mt-2 flex flex-col gap-2">
            {topic.sources.map((s) => (
              <li
                key={`${s.document_title}-${s.utterance_no}`}
                className="text-sm"
              >
                <p className="text-xs text-zinc-500">
                  発言{s.utterance_no} {s.speaker}
                </p>
                <blockquote className="rounded-xl bg-zinc-100 px-3 py-2 leading-6 dark:bg-zinc-800">
                  「{s.quote}」
                </blockquote>
                <p className="mt-0.5 text-xs text-zinc-500">
                  {s.url ? (
                    <a
                      href={s.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="underline underline-offset-2"
                    >
                      {s.document_title}
                    </a>
                  ) : (
                    s.document_title
                  )}
                  {s.held_on && `（${s.held_on}）`}
                </p>
              </li>
            ))}
          </ul>
        </section>
      )}

      <section aria-label="最初の意見" className="mt-4">
        <h3 className="text-xs font-semibold text-zinc-600 dark:text-zinc-400">
          最初の意見（{topic.statements.length}）
        </h3>
        <ul className="mt-2 flex flex-col divide-y divide-zinc-200 dark:divide-zinc-800">
          {topic.statements.map((s) => (
            <StatementRow
              key={s.id}
              slug={slug}
              statement={s}
              onChanged={reload}
            />
          ))}
        </ul>
        <AddStatement slug={slug} topic={topic} onChange={onChange} />
      </section>

      <TagPicker
        slug={slug}
        topic={topic}
        choices={tagChoices}
        onChange={onChange}
      />

      {error && <p className={`mt-3 ${ERROR_TEXT}`}>{error}</p>}
      <div className="mt-4 flex gap-2">
        {/* ふだんは公開待ちに入れ、決まった時刻に古い順に公開する。急ぐものだけ今すぐ公開する */}
        <button
          type="button"
          disabled={busy}
          onClick={() => void enqueue()}
          className={`h-10 flex-1 text-sm ${BUTTON_PRIMARY}`}
        >
          公開待ちに入れる
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => setStatus("publish")}
          className={`h-10 px-4 text-sm ${BUTTON_SECONDARY}`}
        >
          今すぐ公開
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => setStatus("archive")}
          className={`h-10 px-4 text-sm ${BUTTON_SECONDARY}`}
        >
          取り下げる
        </button>
      </div>
    </article>
  );
}

function TitleEditor({
  slug,
  topic,
  onChange,
}: {
  slug: string;
  topic: OperatorTopic;
  onChange: (topic: OperatorTopic) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(topic.title);
  const [description, setDescription] = useState(topic.description);
  const { busy, error, run } = useAction();

  if (!editing) {
    return (
      <>
        {/* 出典・意見・タグと照らしながら読めるよう、カードの終わりまでタブの下に貼り付ける */}
        <div className={`mt-1 ${STICKY_CARD_HEAD}`}>
          <h2 className="text-base font-semibold">{topic.title}</h2>
          <p className="mt-1 text-sm leading-6 whitespace-pre-wrap text-zinc-700 dark:text-zinc-300">
            {topic.description}
          </p>
        </div>
        <button
          type="button"
          onClick={() => {
            setTitle(topic.title);
            setDescription(topic.description);
            setEditing(true);
          }}
          className={`mt-2 ${SMALL_BUTTON}`}
        >
          タイトル・説明を直す
        </button>
      </>
    );
  }

  async function save() {
    const ok = await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(operatorTopicApi(slug, topic.id), {
            method: "PATCH",
            body: { title, description },
          }),
        ),
      "直せませんでした",
    );
    if (ok) setEditing(false);
  }

  return (
    <div className="mt-2 flex flex-col gap-2">
      <label className="text-xs text-zinc-500">
        タイトル
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          className={`mt-1 ${INPUT}`}
        />
      </label>
      <label className="text-xs text-zinc-500">
        説明（{description.length}字）
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={6}
          className={`mt-1 ${INPUT}`}
        />
      </label>
      {error && <p className={ERROR_TEXT}>{error}</p>}
      <div className="flex gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={save}
          className={`h-8 px-4 text-xs ${BUTTON_PRIMARY}`}
        >
          保存
        </button>
        <button
          type="button"
          onClick={() => setEditing(false)}
          className={SMALL_BUTTON}
        >
          やめる
        </button>
      </div>
    </div>
  );
}

function StatementRow({
  slug,
  statement,
  onChanged,
}: {
  slug: string;
  statement: OperatorStatement;
  onChanged: () => Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [body, setBody] = useState(statement.body);
  const { busy, error, run } = useAction();
  const api = operatorStatementApi(slug, statement.id);

  async function save() {
    const ok = await run(async () => {
      await apiFetch(api, { method: "PATCH", body: { body } });
      await onChanged();
    }, "直せませんでした");
    if (ok) setEditing(false);
  }

  async function remove() {
    if (!window.confirm(`「${statement.body}」を取り下げます。よろしいですか`))
      return;
    await run(async () => {
      await apiFetch(`${api}/reject`, { method: "POST" });
      await onChanged();
    }, "取り下げられませんでした");
  }

  return (
    <li className="py-2">
      {editing ? (
        <BodyInput value={body} onChange={setBody} />
      ) : (
        <p className="text-sm leading-6">
          {statement.status !== "approved" && (
            <span
              className={`mr-1 rounded px-1.5 py-0.5 text-xs ${BADGE_AMBER}`}
            >
              {STATEMENT_STATUS_LABELS[statement.status]}
            </span>
          )}
          {statement.body}
        </p>
      )}
      {statement.basis.length > 0 && (
        <p className="mt-0.5 text-xs text-zinc-500">
          根拠: {statement.basis.join("、")}
        </p>
      )}
      {error && <p className={`mt-1 ${ERROR_TEXT}`}>{error}</p>}
      <div className="mt-1.5 flex gap-2">
        {editing ? (
          <>
            <button
              type="button"
              disabled={busy}
              onClick={save}
              className={`h-8 px-4 text-xs ${BUTTON_PRIMARY}`}
            >
              保存
            </button>
            <button
              type="button"
              onClick={() => setEditing(false)}
              className={SMALL_BUTTON}
            >
              やめる
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              onClick={() => {
                setBody(statement.body);
                setEditing(true);
              }}
              className={SMALL_BUTTON}
            >
              直す
            </button>
            <button
              type="button"
              disabled={busy}
              onClick={remove}
              className={SMALL_BUTTON}
            >
              取り下げる
            </button>
          </>
        )}
      </div>
    </li>
  );
}

function BodyInput({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}) {
  const chars = countChars(value);
  const over = chars > MAX_STATEMENT_CHARS;
  return (
    <div>
      <textarea
        value={value}
        onChange={(e) => onChange(e.target.value)}
        rows={3}
        placeholder={placeholder}
        aria-label="意見の本文"
        className={INPUT}
      />
      <p
        className={`text-right text-xs ${over ? "text-rose-600" : "text-zinc-500"}`}
      >
        {chars} / {MAX_STATEMENT_CHARS}
      </p>
    </div>
  );
}

function AddStatement({
  slug,
  topic,
  onChange,
}: {
  slug: string;
  topic: OperatorTopic;
  onChange: (topic: OperatorTopic) => void;
}) {
  const [body, setBody] = useState("");
  const { busy, error, run } = useAction();

  async function add() {
    const ok = await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(
            `${operatorTopicApi(slug, topic.id)}/statements`,
            { method: "POST", body: { body } },
          ),
        ),
      "足せませんでした",
    );
    if (ok) setBody("");
  }

  return (
    <div className="mt-2">
      <BodyInput
        value={body}
        onChange={setBody}
        placeholder="最初の意見を足す（議会で実際に出た主張から）"
      />
      {error && <p className={ERROR_TEXT}>{error}</p>}
      <button
        type="button"
        disabled={busy || body.trim().length === 0}
        onClick={add}
        className={SMALL_BUTTON}
      >
        足す
      </button>
    </div>
  );
}

function TagPicker({
  slug,
  topic,
  choices,
  onChange,
}: {
  slug: string;
  topic: OperatorTopic;
  choices: TagChoice[];
  onChange: (topic: OperatorTopic) => void;
}) {
  const { busy, error, run } = useAction();
  const assigned = new Map(topic.tags.map((t) => [t.id, t.assigned_by]));

  async function toggle(tag: TagChoice) {
    await run(
      async () =>
        onChange(
          await apiFetch<OperatorTopic>(
            `${operatorTopicApi(slug, topic.id)}/tags/${encodeURIComponent(tag.id)}`,
            { method: assigned.has(tag.id) ? "DELETE" : "PUT" },
          ),
        ),
      "タグを変えられませんでした",
    );
  }

  const groups = [
    { label: "分野", tags: choices.filter((t) => t.kind === "field") },
    { label: "地区", tags: choices.filter((t) => t.kind === "district") },
  ];
  return (
    <section aria-label="タグ" className="mt-4">
      <h3 className="text-xs font-semibold text-zinc-600 dark:text-zinc-400">
        タグ（AI が付けたものに「AI」）
      </h3>
      {groups.map((g) => (
        <div key={g.label} className="mt-2">
          <p className="text-xs text-zinc-500">{g.label}</p>
          <div className="mt-1 flex flex-wrap gap-1.5">
            {g.tags.map((tag) => (
              <button
                key={tag.id}
                type="button"
                disabled={busy}
                aria-pressed={assigned.has(tag.id)}
                onClick={() => toggle(tag)}
                className={chipClass(assigned.has(tag.id))}
              >
                {tag.name}
                {assigned.get(tag.id) === "ai" && (
                  <span className="ml-1 text-[10px] opacity-70">AI</span>
                )}
              </button>
            ))}
          </div>
        </div>
      ))}
      {error && <p className={`mt-1 ${ERROR_TEXT}`}>{error}</p>}
    </section>
  );
}

/** 似ているトピック。同じ論点なら、取り下げるか、意見をそちらに足すかを運営者が決める */
function SimilarTopics({ topic }: { topic: OperatorTopic }) {
  if (topic.similar.length === 0) return null;
  return (
    <section aria-label="似ているトピック" className="mt-3">
      <h3 className="text-xs font-semibold text-zinc-600 dark:text-zinc-400">
        似ているトピック（同じ論点でないか確かめてください）
      </h3>
      <ul className="mt-1 flex flex-col gap-1 text-sm">
        {topic.similar.map((t) => (
          <li key={t.id} className="flex items-baseline gap-2">
            <span
              className={`shrink-0 rounded px-1.5 py-0.5 text-xs ${BADGE_AMBER}`}
            >
              {t.status === "draft" ? "下書き" : "公開中"}
            </span>
            <span className="min-w-0 flex-1">{t.title}</span>
            <span className="shrink-0 text-xs tabular-nums text-zinc-500">
              {Math.round(t.score * 100)}%
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
