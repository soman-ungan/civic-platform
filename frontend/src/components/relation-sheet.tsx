// SPDX-License-Identifier: AGPL-3.0-only

"use client";

import { useState } from "react";

import { ApiError, apiFetch } from "@/lib/api-client";
import { RESIDENCY_LABELS } from "@/lib/labels";
import type { District, ResidencyType } from "@/lib/types";

const RESIDENCY_TYPES = Object.keys(RESIDENCY_LABELS) as ResidencyType[];

/** その自治体で初めて投票する前に、1回だけ自治体との関係を聞くボトムシート */
export function RelationSheet({
  slug,
  districts,
  onClose,
  onSaved,
}: {
  slug: string;
  districts: District[];
  onClose: () => void;
  onSaved: () => void;
}) {
  const [residency, setResidency] = useState<ResidencyType | null>(null);
  const [districtId, setDistrictId] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: React.FormEvent) {
    event.preventDefault();
    if (!residency) return;
    setSaving(true);
    setError(null);
    try {
      await apiFetch(
        `/api/municipalities/${encodeURIComponent(slug)}/relation`,
        {
          method: "PUT",
          body: {
            residency_type: residency,
            district_id:
              residency === "resident" && districtId ? districtId : null,
          },
        },
      );
      onSaved();
    } catch (e) {
      setError(e instanceof ApiError ? e.message : "保存に失敗しました");
      setSaving(false);
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-end justify-center bg-black/40"
      onClick={onClose}
    >
      <form
        role="dialog"
        aria-modal="true"
        aria-labelledby="relation-title"
        onSubmit={submit}
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-md rounded-t-3xl bg-white px-5 pt-5 pb-[max(1.25rem,env(safe-area-inset-bottom))] dark:bg-zinc-900"
      >
        <h2 id="relation-title" className="text-lg font-bold">
          このまちとの関わりを教えてください
        </h2>
        <p className="mt-1 text-xs leading-5 text-zinc-500">
          最初の1回だけ聞きます。住民と関係人口などで意見の傾向を比べるために使い、個人が分かる形では公開しません。
        </p>

        <fieldset className="mt-4 flex flex-col gap-2">
          <legend className="sr-only">関わり方</legend>
          {RESIDENCY_TYPES.map((type) => (
            <label
              key={type}
              className={`flex min-h-12 items-center gap-3 rounded-xl border px-4 text-sm ${
                residency === type
                  ? "border-zinc-900 dark:border-zinc-100"
                  : "border-zinc-200 dark:border-zinc-700"
              }`}
            >
              <input
                type="radio"
                name="residency"
                value={type}
                checked={residency === type}
                onChange={() => setResidency(type)}
              />
              {RESIDENCY_LABELS[type]}
            </label>
          ))}
        </fieldset>

        {residency === "resident" && districts.length > 0 && (
          <label className="mt-4 block text-sm">
            お住まいの地区（任意）
            <select
              value={districtId}
              onChange={(e) => setDistrictId(e.target.value)}
              className="mt-1 h-12 w-full rounded-xl border border-zinc-200 bg-white px-3 dark:border-zinc-700 dark:bg-zinc-900"
            >
              <option value="">選ばない</option>
              {districts.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </select>
          </label>
        )}

        {error && <p className="mt-3 text-sm text-rose-600">{error}</p>}

        <div className="mt-5 flex gap-2">
          <button
            type="button"
            onClick={onClose}
            className="h-12 flex-1 rounded-full border border-zinc-300 text-sm dark:border-zinc-700"
          >
            あとで
          </button>
          <button
            type="submit"
            disabled={!residency || saving}
            className="h-12 flex-[2] rounded-full bg-zinc-900 text-sm font-medium text-white disabled:opacity-40 dark:bg-zinc-100 dark:text-zinc-900"
          >
            {saving ? "保存中…" : "保存して投票する"}
          </button>
        </div>
      </form>
    </div>
  );
}
