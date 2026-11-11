// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { EmptyState } from "@/components/ui/empty-state";
import {
  BUTTON_PRIMARY,
  BUTTON_SECONDARY,
  ERROR_TEXT,
} from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorApi, operatorTopicApi } from "@/lib/api/paths";
import type {
  OperatorTopic,
  PublishSlot,
  Publishing,
} from "@/lib/api/types/operator";
import { useApi } from "@/lib/api/use-api";
import { SOURCE_LABELS } from "@/lib/labels";
import { formatDateTime } from "@/lib/operator";

import { STICKY_SECTION_HEAD } from "./sticky";
import { useAction } from "./use-action";

const INPUT =
  "h-10 rounded-lg border border-zinc-300 bg-white px-3 text-sm dark:border-zinc-700 dark:bg-zinc-900";
const SMALL_BUTTON = `h-10 px-3 text-xs ${BUTTON_SECONDARY}`;

/**
 * 公開待ち（docs/plans/2026-10-30-timeline-flow.md）: 公開の時刻（日本時間。1つで1日1件）と、
 * 公開する順（先に入れたものから。上へ・下へで入れ替える）。時刻はトピックの数に合わせて足す・消す・変える
 */
export function Scheduled({ slug }: { slug: string }) {
  const api = `${operatorApi(slug)}/publishing`;
  const { data, error, reload, update } = useApi<Publishing>(api);

  if (!data) {
    return error ? (
      <p className={ERROR_TEXT}>読み込めませんでした</p>
    ) : (
      <p className="text-sm text-zinc-500">読み込み中…</p>
    );
  }
  return (
    <>
      <Slots
        slug={slug}
        slots={data.slots}
        onChange={(next) => update(() => next)}
      />
      <section aria-labelledby="queue-heading" className="mt-8">
        <h2
          id="queue-heading"
          className={`text-sm font-semibold ${STICKY_SECTION_HEAD}`}
        >
          公開する順（{data.queue.length}件）
        </h2>
        {data.queue.length === 0 ? (
          <div className="mt-3">
            <EmptyState>
              公開待ちはありません。下書きのタブで「公開待ちに入れる」と、ここに並びます
            </EmptyState>
          </div>
        ) : (
          <ol className="mt-3 flex flex-col gap-3">
            {data.queue.map((topic, i) => (
              <QueuedCard
                key={topic.id}
                slug={slug}
                order={i + 1}
                last={i === data.queue.length - 1}
                topic={topic}
                onChange={() => void reload()}
                onMoved={(next) => update(() => next)}
              />
            ))}
          </ol>
        )}
      </section>
    </>
  );
}

/** 公開の時刻の設定 */
function Slots({
  slug,
  slots,
  onChange,
}: {
  slug: string;
  slots: PublishSlot[];
  onChange: (next: Publishing) => void;
}) {
  const api = `${operatorApi(slug)}/publish-slots`;
  const { busy, error, run } = useAction();
  const [time, setTime] = useState("07:00");

  async function send(
    path: string,
    method: "POST" | "PATCH" | "DELETE",
    timeOfDay?: string,
  ) {
    await run(
      async () =>
        onChange(
          await apiFetch<Publishing>(path, {
            method,
            body:
              timeOfDay === undefined ? undefined : { time_of_day: timeOfDay },
          }),
        ),
      "変えられませんでした",
    );
  }

  return (
    <section aria-labelledby="slots-heading">
      <h2 id="slots-heading" className="text-sm font-semibold">
        公開の時刻（日本時間）
      </h2>
      <p className="mt-1 text-xs leading-5 text-zinc-500">
        時刻ごとに、公開待ちの先頭から1件ずつ公開します（時刻が1つで1日1件）。今日の時刻を過ぎてから足した・変えた時刻は、明日から動きます。
      </p>
      {slots.length === 0 && (
        <p className="mt-3 text-sm text-amber-700 dark:text-amber-400">
          時刻がありません。足すまで、公開待ちは公開されません。
        </p>
      )}
      <ul className="mt-3 flex flex-col gap-2">
        {slots.map((slot) => (
          <SlotRow
            key={slot.id}
            slot={slot}
            busy={busy}
            onSave={(t) => void send(`${api}/${slot.id}`, "PATCH", t)}
            onRemove={() => {
              if (
                window.confirm(
                  `${slot.time_of_day} の公開をやめます。よろしいですか`,
                )
              )
                void send(`${api}/${slot.id}`, "DELETE");
            }}
          />
        ))}
      </ul>
      <form
        className="mt-3 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          void send(api, "POST", time);
        }}
      >
        <label htmlFor="new-slot" className="sr-only">
          足す時刻
        </label>
        <input
          id="new-slot"
          type="time"
          required
          value={time}
          onChange={(e) => setTime(e.target.value)}
          className={INPUT}
        />
        <button
          type="submit"
          disabled={busy || !time}
          className={`h-10 px-4 text-sm ${BUTTON_PRIMARY}`}
        >
          時刻を足す
        </button>
      </form>
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
    </section>
  );
}

function SlotRow({
  slot,
  busy,
  onSave,
  onRemove,
}: {
  slot: PublishSlot;
  busy: boolean;
  onSave: (time: string) => void;
  onRemove: () => void;
}) {
  const [time, setTime] = useState(slot.time_of_day);
  const changed = time !== slot.time_of_day;
  return (
    <li className="flex items-center gap-2">
      <label htmlFor={`slot-${slot.id}`} className="sr-only">
        公開の時刻
      </label>
      <input
        id={`slot-${slot.id}`}
        type="time"
        value={time}
        onChange={(e) => setTime(e.target.value)}
        className={INPUT}
      />
      {changed && (
        <button
          type="button"
          disabled={busy || !time}
          onClick={() => onSave(time)}
          className={SMALL_BUTTON}
        >
          変える
        </button>
      )}
      <button
        type="button"
        disabled={busy}
        onClick={onRemove}
        className={`ml-auto ${SMALL_BUTTON}`}
      >
        消す
      </button>
    </li>
  );
}

/** 公開待ちの1件: 予定・順の入れ替え・外す（下書きに戻す）・今すぐ公開 */
function QueuedCard({
  slug,
  order,
  last,
  topic,
  onChange,
  onMoved,
}: {
  slug: string;
  order: number;
  last: boolean;
  topic: Publishing["queue"][number];
  onChange: () => void;
  onMoved: (next: Publishing) => void;
}) {
  const api = operatorTopicApi(slug, topic.id);
  const { busy, error, run } = useAction();

  async function act(
    path: string,
    method: "POST" | "DELETE",
    message?: string,
  ) {
    if (message && !window.confirm(message)) return;
    if (
      await run(
        () => apiFetch<OperatorTopic>(path, { method }),
        "変えられませんでした",
      )
    )
      onChange();
  }

  async function move(direction: "up" | "down") {
    await run(
      async () =>
        onMoved(
          await apiFetch<Publishing>(`${api}/queue/move`, {
            method: "POST",
            body: { direction },
          }),
        ),
      "入れ替えられませんでした",
    );
  }

  return (
    <li className="rounded-2xl border border-zinc-200 p-4 dark:border-zinc-800">
      <p className="text-xs text-zinc-500">
        {order}番目 ・ {SOURCE_LABELS[topic.source_type]} ・{" "}
        {topic.planned_at
          ? `予定 ${formatDateTime(topic.planned_at)}`
          : "時刻が無いので予定なし"}
      </p>
      <p className="mt-1 font-semibold leading-snug">{topic.title}</p>
      {error && <p className={`mt-2 ${ERROR_TEXT}`}>{error}</p>}
      <div className="mt-3 flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy || order === 1}
          onClick={() => void move("up")}
          aria-label={`「${topic.title}」を1つ前へ`}
          className={SMALL_BUTTON}
        >
          ↑ 前へ
        </button>
        <button
          type="button"
          disabled={busy || last}
          onClick={() => void move("down")}
          aria-label={`「${topic.title}」を1つ後ろへ`}
          className={SMALL_BUTTON}
        >
          ↓ 後ろへ
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() => void act(`${api}/queue`, "DELETE")}
          className={SMALL_BUTTON}
        >
          外す（下書きに戻す）
        </button>
        <button
          type="button"
          disabled={busy}
          onClick={() =>
            void act(
              `${api}/publish`,
              "POST",
              `「${topic.title}」を今すぐ公開します。公開から90日の期限が付きます。よろしいですか`,
            )
          }
          className={`ml-auto ${SMALL_BUTTON}`}
        >
          今すぐ公開
        </button>
      </div>
    </li>
  );
}
