// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useState } from "react";

import {
  PASSKEY_EXPLANATION,
  PasskeyChoices,
} from "@/components/auth/passkey-choices";
import { RelationSheet } from "@/components/participation/relation-sheet";
import { BUTTON_PRIMARY, ERROR_TEXT } from "@/components/ui/styles";
import { ApiError, apiFetch, errorMessage } from "@/lib/api/client";
import { proposalsApi } from "@/lib/api/paths";
import type {
  Proposal,
  ProposalState,
  Proposals,
  ProposePermission,
} from "@/lib/api/types/proposals";
import { useApi } from "@/lib/api/use-api";
import { formatDate } from "@/lib/dates";
import {
  MAX_DESCRIPTION_CHARS,
  MAX_STATEMENT_CHARS,
  MAX_TITLE_CHARS,
  countChars,
} from "@/lib/statement";

/** 提案に含められる最初の意見の数（backend の `progress::PROPOSAL_SEEDS`。ADR 0005） */
const MAX_PROPOSAL_STATEMENTS = 10;

/** 提案した人が、公開のあとに最初の意見とは別に投稿できる意見の数（`STATEMENTS_PER_TOPIC_PROPOSER`） */
const PROPOSER_POSTS = 3;

const INPUT =
  "mt-2 w-full rounded-xl border border-zinc-200 bg-white p-3 text-base leading-6 dark:border-zinc-700 dark:bg-zinc-900";

/**
 * トピックの提案（ADR 0005）: 提案のフォームと「自分の提案」の一覧。
 * 提案できるかは backend の `permission` を見る（パスキーの登録・関わり方・自治体ごとに週1件。ルールは写さない）
 */
export function ProposalPanel({
  slug,
  initial,
  from,
}: {
  slug: string;
  initial: Proposals;
  /** 市民のまとめから提案するとき: 元のトピック（起点が先）と、最初の意見に使う原文 */
  from?: ProposalSource;
}) {
  const api = proposalsApi(slug);
  const { data, reload } = useApi<Proposals>(api, undefined, initial);
  const proposals = data ?? initial;
  const [done, setDone] = useState(false);

  return (
    <>
      {done && (
        <p
          role="status"
          className="mb-4 rounded-2xl bg-emerald-50 p-4 text-sm leading-6 text-emerald-800 dark:bg-emerald-950/40 dark:text-emerald-300"
        >
          提案を受け付けました。運営者が確かめてから公開します。結果は下の「自分の提案」に出ます。
        </p>
      )}
      <PermissionGate
        slug={slug}
        permission={proposals.permission}
        onChanged={() => void reload()}
      >
        <ProposalForm
          api={api}
          from={from}
          onSubmitted={() => {
            setDone(true);
            void reload();
          }}
          onRejected={() => void reload()}
        />
      </PermissionGate>
      <MyProposals slug={slug} items={proposals.items} />
    </>
  );
}

/** 提案できないときは、その理由と次にすること。できるときはフォーム */
function PermissionGate({
  slug,
  permission,
  onChanged,
  children,
}: {
  slug: string;
  permission: ProposePermission;
  onChanged: () => void;
  children: React.ReactNode;
}) {
  const [relationOpen, setRelationOpen] = useState(false);
  const router = useRouter();

  switch (permission.status) {
    case "allowed":
      return children;
    case "registration_required":
      return (
        <div className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
          <p className="text-sm font-medium">
            トピックを提案するには、パスキーでのログインが要ります
          </p>
          <p className="mt-1 text-xs leading-5 text-zinc-500">
            {PASSKEY_EXPLANATION}これまでの投票はそのまま残ります。
          </p>
          <PasskeyChoices
            createLabel="パスキーを作って提案"
            onRegistered={onChanged}
            secondary={{
              label: "やめる",
              onClick: () => router.push(`/${slug}`),
            }}
          />
        </div>
      );
    case "relation_required":
      return (
        <div className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
          <p className="text-sm leading-6">
            提案の前に、このまちとの関わりを教えてください。
          </p>
          <button
            type="button"
            onClick={() => setRelationOpen(true)}
            className={`mt-3 h-11 w-full text-sm ${BUTTON_PRIMARY}`}
          >
            関わりを登録する
          </button>
          {relationOpen && (
            <RelationSheet
              slug={slug}
              submitLabel="保存して提案する"
              onClose={() => setRelationOpen(false)}
              onSaved={() => {
                setRelationOpen(false);
                onChanged();
              }}
            />
          )}
        </div>
      );
    case "weekly_limit":
      return (
        <p className="rounded-2xl border border-dashed border-zinc-300 p-4 text-center text-xs leading-5 text-zinc-500 dark:border-zinc-700">
          提案は1週間に1件までです。次に提案できるのは
          {formatDate(permission.next_at)}からです。
        </p>
      );
  }
}

/** 市民のまとめから提案するときに、フォームへ渡すもの */
export type ProposalSource = {
  parents: { id: string; title: string; description: string }[];
  statements: string[];
};

function ProposalForm({
  api,
  from,
  onSubmitted,
  onRejected,
}: {
  api: string;
  from?: ProposalSource;
  onSubmitted: () => void;
  /** 週1件などで断られた（`permission` を読み直す） */
  onRejected: () => void;
}) {
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [statements, setStatements] = useState<string[]>(
    from && from.statements.length > 0
      ? from.statements.slice(0, MAX_PROPOSAL_STATEMENTS)
      : [""],
  );
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const titleCount = countChars(title.trim());
  const descriptionCount = countChars(description.trim());
  const filled = statements.filter((s) => s.trim() !== "");
  const ready =
    titleCount > 0 &&
    titleCount <= MAX_TITLE_CHARS &&
    descriptionCount > 0 &&
    descriptionCount <= MAX_DESCRIPTION_CHARS &&
    filled.length > 0 &&
    statements.every((s) => countChars(s.trim()) <= MAX_STATEMENT_CHARS);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!ready) return;
    setSending(true);
    setError(null);
    try {
      await apiFetch<Proposal>(api, {
        method: "POST",
        body: {
          title,
          description,
          statements: filled,
          parent_ids: from?.parents.map((p) => p.id) ?? [],
        },
      });
      setTitle("");
      setDescription("");
      setStatements([""]);
      onSubmitted();
    } catch (e) {
      setError(errorMessage(e, "提案を送れませんでした"));
      if (e instanceof ApiError && e.status !== 422) onRejected();
    } finally {
      setSending(false);
    }
  }

  return (
    <form
      onSubmit={submit}
      className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800"
    >
      {from && from.parents.length > 0 && (
        <div className="mb-4 rounded-xl bg-zinc-50 p-3 dark:bg-zinc-900">
          <p className="text-xs font-medium">元のトピック（まとめから）</p>
          {/* 題を押すと説明が開く（元の論点を見ながら書けるように） */}
          <ul className="mt-1 flex flex-col gap-1">
            {from.parents.map((p) => (
              <li key={p.id}>
                <details className="group text-xs leading-5">
                  <summary className="cursor-pointer list-none text-zinc-700 dark:text-zinc-300">
                    <span
                      aria-hidden
                      className="mr-1 inline-block text-zinc-400 transition-transform group-open:rotate-90"
                    >
                      ▶
                    </span>
                    {p.title}
                  </summary>
                  <p className="mt-1 pl-4 text-zinc-600 dark:text-zinc-400">
                    {p.description || "（説明はありません）"}
                  </p>
                </details>
              </li>
            ))}
          </ul>
          <p className="mt-2 text-xs leading-5 text-zinc-500">
            公開されると、元のトピックからこのトピックをたどれるようになります。
          </p>
        </div>
      )}
      <p className="text-xs leading-5 text-zinc-500">
        運営者が確かめてから公開します。公開されるまで、ほかの人には見えません。提案は1つのまちに1週間に1件までです。提案した人が誰かは、公開しません。
      </p>

      <Field
        id="proposal-title"
        label="タイトル"
        count={titleCount}
        max={MAX_TITLE_CHARS}
      >
        <input
          id="proposal-title"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          className={INPUT}
        />
      </Field>

      <Field
        id="proposal-description"
        label="説明"
        note="何が課題で、どんな選択肢がありそうかを、短く書いてください。あなたの考えは、下の「最初の意見」に書きます。"
        count={descriptionCount}
        max={MAX_DESCRIPTION_CHARS}
      >
        <textarea
          id="proposal-description"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={3}
          className={`${INPUT} resize-none`}
        />
      </Field>

      <fieldset className="mt-5">
        <legend className="text-sm font-medium">
          最初の意見（1〜{MAX_PROPOSAL_STATEMENTS}件）
        </legend>
        <p className="mt-1 text-xs leading-5 text-zinc-500">
          参加する人が、まず投票する意見です。ほかの人が賛成・反対できる意見を、1件に1つずつ書いてください。あなたと違う立場の意見も書けます（その立場の人が読んで納得できる書き方で）。人ではなく、政策や事業についての意見にしてください。
        </p>
        <p className="mt-1 text-xs leading-5 text-zinc-500">
          いろいろな立場の意見がそろっているほど、意見の分かれ方がはっきりします。公開されたら、あなたもほかの人と同じように、この意見に賛成・反対・パスで投票します。
        </p>
        {statements.map((body, i) => (
          <div key={i} className="mt-3">
            <label htmlFor={`proposal-statement-${i}`} className="sr-only">
              意見{i + 1}
            </label>
            <textarea
              id={`proposal-statement-${i}`}
              value={body}
              onChange={(e) =>
                setStatements((list) =>
                  list.map((s, j) => (j === i ? e.target.value : s)),
                )
              }
              rows={2}
              className={`${INPUT} mt-0 resize-none`}
            />
            <div className="mt-1 flex items-center justify-between">
              <Count
                count={countChars(body.trim())}
                max={MAX_STATEMENT_CHARS}
              />
              {statements.length > 1 && (
                <button
                  type="button"
                  onClick={() =>
                    setStatements((list) => list.filter((_, j) => j !== i))
                  }
                  className="h-8 px-2 text-xs text-zinc-500"
                >
                  消す
                </button>
              )}
            </div>
          </div>
        ))}
        {statements.length < MAX_PROPOSAL_STATEMENTS && (
          <button
            type="button"
            onClick={() => setStatements((list) => [...list, ""])}
            className="mt-2 h-11 w-full rounded-full border border-dashed border-zinc-400 text-sm text-zinc-700 dark:border-zinc-600 dark:text-zinc-300"
          >
            ＋ 意見を足す
          </button>
        )}
        <p className="mt-2 text-xs text-zinc-500">
          公開のあと、最初の意見とは別に、あと{PROPOSER_POSTS}
          件の意見を投稿できます。
        </p>
      </fieldset>

      <p className="mt-5 text-xs leading-5 text-zinc-500">
        公開したあと、分野のタグを付けるために、タイトルと説明を AI（Anthropic
        の Claude）に送ります。あなたを識別する情報は送りません。
      </p>
      {error && <p className={`mt-3 ${ERROR_TEXT}`}>{error}</p>}
      <button
        type="submit"
        disabled={!ready || sending}
        className={`mt-4 h-12 w-full text-sm ${BUTTON_PRIMARY}`}
      >
        {sending ? "送っています…" : "提案する"}
      </button>
    </form>
  );
}

function Field({
  id,
  label,
  note,
  count,
  max,
  children,
}: {
  id: string;
  label: string;
  note?: string;
  count: number;
  max: number;
  children: React.ReactNode;
}) {
  return (
    <div className="mt-5">
      <label htmlFor={id} className="text-sm font-medium">
        {label}（{max}文字まで）
      </label>
      {note && <p className="mt-1 text-xs leading-5 text-zinc-500">{note}</p>}
      {children}
      <Count count={count} max={max} />
    </div>
  );
}

function Count({ count, max }: { count: number; max: number }) {
  return (
    <span
      className={`text-xs ${count > max ? "font-medium text-rose-600" : "text-zinc-500"}`}
    >
      {count} / {max}
    </span>
  );
}

const STATE_LABELS: Record<ProposalState, string> = {
  reviewing: "確認中",
  published: "公開中",
  declined: "見送り",
  withdrawn: "取り下げ",
};

/** 自分の提案（提案した本人にだけ見える） */
function MyProposals({ slug, items }: { slug: string; items: Proposal[] }) {
  if (items.length === 0) return null;
  return (
    <section aria-labelledby="my-proposals" className="mt-8">
      <h2 id="my-proposals" className="text-base font-bold">
        自分の提案
      </h2>
      <ul className="mt-3 flex flex-col gap-3">
        {items.map((p) => (
          <li
            key={p.id}
            className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800"
          >
            <p className="text-xs text-zinc-500">
              {formatDate(p.created_at)} ・ {STATE_LABELS[p.state]}
            </p>
            {p.state === "published" ? (
              <Link
                href={`/${slug}/topics/${encodeURIComponent(p.id)}`}
                className="mt-1 block font-semibold underline underline-offset-4"
              >
                {p.title}
              </Link>
            ) : (
              <p className="mt-1 font-semibold">{p.title}</p>
            )}
            <p className="mt-1 text-sm leading-6 text-zinc-600 dark:text-zinc-400">
              {p.description}
            </p>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** ISO8601 → 「2026年10月29日」。サーバーとブラウザで同じになるよう、日本時間で表示する */
