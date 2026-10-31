// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";

/**
 * このサービスについて・利用規約・プライバシーポリシー・アカウントのページの枠。
 * 自治体によらないページなので、戻る先はトップ（有効な自治体が1つならそのトップへ移る）
 */
export function InfoPage({
  title,
  updated,
  children,
}: {
  title: string;
  /** 制定・改定の日（例「2026年10月31日」） */
  updated?: string;
  children: React.ReactNode;
}) {
  return (
    <main className="min-h-0 flex-1 overflow-y-auto">
      <article className="mx-auto w-full max-w-md px-4 pt-4 pb-10 text-sm leading-7 text-zinc-700 dark:text-zinc-300">
        <Link
          href="/"
          className="text-xs text-zinc-500 underline underline-offset-2"
        >
          ← トップへ
        </Link>
        <h1 className="mt-3 text-xl font-bold tracking-tight text-zinc-900 dark:text-zinc-100">
          {title}
        </h1>
        {children}
        {updated && (
          <p className="mt-10 text-xs text-zinc-500">{updated} 制定</p>
        )}
        <nav
          aria-label="関連するページ"
          className="mt-6 flex flex-wrap gap-x-4 gap-y-1 text-xs text-zinc-500"
        >
          {INFO_LINKS.map((l) => (
            <Link
              key={l.href}
              href={l.href}
              className="underline underline-offset-2"
            >
              {l.label}
            </Link>
          ))}
        </nav>
      </article>
    </main>
  );
}

export const INFO_LINKS = [
  { href: "/about", label: "このサービスについて" },
  { href: "/terms", label: "利用規約" },
  { href: "/privacy", label: "プライバシーポリシー" },
] as const;

/** 節（見出しと本文） */
export function Section({
  id,
  title,
  children,
}: {
  id?: string;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section id={id} className="mt-8 scroll-mt-4">
      <h2 className="mb-2 font-semibold text-zinc-900 dark:text-zinc-100">
        {title}
      </h2>
      <div className="flex flex-col gap-3">{children}</div>
    </section>
  );
}

/** 箇条書き */
export function List({ children }: { children: React.ReactNode }) {
  return <ul className="list-disc space-y-1 pl-5">{children}</ul>;
}

/** 運営者が書き足すところ（公開の前に埋める） */
export function ToFill({ children }: { children: React.ReactNode }) {
  return (
    <span className="rounded bg-amber-100 px-1 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100">
      【要記入: {children}】
    </span>
  );
}

/**
 * 問い合わせ先のメールアドレス（`.env` の `NEXT_PUBLIC_CONTACT_EMAIL`）。
 * 未設定なら、まだ受け付けていないと書く
 */
export function ContactEmail() {
  const email = process.env.NEXT_PUBLIC_CONTACT_EMAIL;
  if (!email) {
    return <span>（問い合わせの窓口は準備中です）</span>;
  }
  return (
    <a
      href={`mailto:${email}`}
      className="font-medium underline underline-offset-2"
    >
      {email}
    </a>
  );
}
