// SPDX-License-Identifier: AGPL-3.0-only

import Link from "next/link";
import { connection } from "next/server";
import { Suspense } from "react";

import {
  SITE_FIELDS,
  type SiteField,
  siteSocialLinks,
  siteValue,
} from "@/lib/site";

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

/**
 * 運営者ごとの値（`lib/site.ts`。`.env` から、リクエストのときに読む）。
 * 未設定なら【要記入】の印を出す（公開の前に `.env` で埋める）。`href` があり、値が https の URL ならリンクにする
 * （「まだ公開していません」のような文章はそのまま出す）。
 * ページのほかの部分は静的に作り、ここだけをリクエストのときに埋める
 */
export function SiteValue({
  field,
  href,
}: {
  field: SiteField;
  href?: boolean;
}) {
  return (
    <Suspense fallback={<span className="text-zinc-400">…</span>}>
      <SiteValueAtRequest field={field} href={href} />
    </Suspense>
  );
}

async function SiteValueAtRequest({
  field,
  href,
}: {
  field: SiteField;
  href?: boolean;
}) {
  await connection();
  const value = siteValue(field);
  if (!value) {
    return (
      <span className="rounded bg-amber-100 px-1 text-amber-900 dark:bg-amber-900/60 dark:text-amber-100">
        【要記入: .env の {SITE_FIELDS[field]}】
      </span>
    );
  }
  if (field === "contactEmail") {
    return (
      <a
        href={`mailto:${value}`}
        className="font-medium underline underline-offset-2"
      >
        {value}
      </a>
    );
  }
  return href && value.startsWith("https://") ? (
    <a
      href={value}
      target="_blank"
      rel="noopener noreferrer"
      className="underline underline-offset-2"
    >
      {value}
    </a>
  ) : (
    <>{value}</>
  );
}

/** 問い合わせ先のメールアドレス（`.env` の `SITE_CONTACT_EMAIL`） */
export function ContactEmail() {
  return <SiteValue field="contactEmail" />;
}

/**
 * 運営者の SNS などのリンク（`.env` の `SITE_SOCIAL_LINKS`）。無ければ何も出さない。
 * リクエストのときに読むので、足したり外したりしても作り直さなくてよい
 */
export function SocialLinks() {
  return (
    <Suspense fallback={null}>
      <SocialLinksAtRequest />
    </Suspense>
  );
}

async function SocialLinksAtRequest() {
  await connection();
  const links = siteSocialLinks();
  if (links.length === 0) return null;
  return (
    <p>
      SNS:{" "}
      {links.map((link, i) => (
        <span key={link.url}>
          {i > 0 && "・"}
          {/* rel="me": Mastodon などで、このサイトと同じ運営者だと示せる */}
          <a
            href={link.url}
            target="_blank"
            rel="me noopener noreferrer"
            className="font-medium underline underline-offset-2"
          >
            {link.label}
          </a>
        </span>
      ))}
      （お知らせも出します。意見の削除の依頼など、個人に関わるご連絡はメールでお願いします）
    </p>
  );
}
