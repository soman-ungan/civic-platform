// SPDX-License-Identifier: AGPL-3.0-only

"use client";

// 下書きの意見の欄: 1件ずつ直す・取り下げる、足す（運営者の意見として）

import { useState } from "react";

import {
  BADGE_AMBER,
  BUTTON_PRIMARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorStatementApi, operatorTopicApi } from "@/lib/api/paths";
import type {
  OperatorStatement,
  OperatorTopic,
} from "@/lib/api/types/operator";
import { STATEMENT_STATUS_LABELS } from "@/lib/operator";
import { MAX_STATEMENT_CHARS, countChars } from "@/lib/statement";

import { useAction } from "./use-action";
import { INPUT, SMALL_BUTTON } from "./styles";

export function StatementRow({
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

export function AddStatement({
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
