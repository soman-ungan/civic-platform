// SPDX-License-Identifier: AGPL-3.0-only

"use client";

// 下書きのタグの欄: 分野と地区のタグを付ける・外す

import { chipClass } from "@/components/ui/chip";
import { ERROR_TEXT } from "@/components/ui/styles";
import { apiFetch } from "@/lib/api/client";
import { operatorTopicApi } from "@/lib/api/paths";
import type { OperatorTopic, TagChoice } from "@/lib/api/types/operator";

import { useAction } from "./use-action";

export function TagPicker({
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
