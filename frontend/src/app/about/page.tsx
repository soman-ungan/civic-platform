// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import Link from "next/link";

import {
  ContactEmail,
  InfoPage,
  List,
  Section,
  ToFill,
} from "@/components/info/prose";
import { pageMetadata } from "@/lib/metadata";

export const metadata: Metadata = pageMetadata(
  "このサービスについて",
  "自治体の議会・予算・決算を分かりやすく見せ、市民の合意点を見つける場です。",
);

export default function AboutPage() {
  const sourceUrl = process.env.NEXT_PUBLIC_SOURCE_URL;
  return (
    <InfoPage title="このサービスについて" updated="2026年10月31日">
      <p className="mt-3">
        まちの議会で話し合われていることや、まちのお金の使い方を分かりやすく見せ、
        市民どうしの「合意できる点」を見つけるための場です。とくに10〜30代の人が、
        スマートフォンで気軽に参加できることを目指しています。
      </p>

      <Section title="できること">
        <List>
          <li>
            <strong>トピックに投票する</strong>:
            議会で実際に出た主張や、市民から提案された論点（トピック）について、
            一つひとつの意見に「賛成・反対・パス」で答えます。読むことと投票は、ログインなしでできます
          </li>
          <li>
            <strong>みんなの意見を見る</strong>:
            最初の意見すべてに投票すると、意見の近い人どうしのグループと、
            グループをこえて賛成の多い「合意点」が見られます
          </li>
          <li>
            <strong>意見を出す・トピックを提案する</strong>:
            パスキーを作ると、意見を1件出したり、新しいトピックを提案したりできます
          </li>
          <li>
            <strong>これまでの議論をまとめる</strong>:
            締め切ったトピックの合意点や意見が分かれた点を、原文と数字のまま見て、
            次のトピックを提案できます
          </li>
          <li>
            <strong>まちのお金を見る</strong>:
            決算と当初予算を、分野ごと・1人あたりの金額で見られます
          </li>
        </List>
      </Section>

      <Section title="大切にしていること">
        <List>
          <li>
            <strong>人を属性で分けません。</strong>
            年齢・性別・住んでいる地区は集めません。分けて見せるのは、そのまちとの関わり方
            （住民・周辺に住む・通勤通学・関係人口）だけで、それも意見のグループ作りには使いません
          </li>
          <li>
            <strong>コメント欄はありません。</strong>
            言い合いではなく、意見への投票から全体の傾向と合意点を見つけます。
            この仕組みは、台湾などで使われている{" "}
            <a
              href="https://pol.is/"
              target="_blank"
              rel="noopener noreferrer"
              className="underline underline-offset-2"
            >
              Polis
            </a>{" "}
            の考え方と計算の方法にもとづいています
          </li>
          <li>
            <strong>お金で議論を変えません。</strong>
            寄付をしても、投稿できる数や表示の順番は変わりません。
            行政からの出資と、政党・政治団体・候補者からの寄付は受けません。収支は公開します
          </li>
          <li>
            <strong>AI は道具として使います。</strong>
            AI
            は、意見を公開してよいかの確認と、会議録や予算の資料の整理に使います。
            主張や政治的な立場では判断しません。議論のまとめやグループの説明は
            AI に作らせず、 原文と数字のまま見せます
          </li>
          <li>
            <strong>名前を出すのは選挙で選ばれた人だけ。</strong>
            会議録からの引用で名前を出すのは、市長・議員などの公人だけです。職員は職名だけにします
          </li>
        </List>
      </Section>

      <Section title="資料の出典">
        <p>
          議会のトピックは各自治体の会議録、まちのお金は総務省の決算カードと各自治体の当初予算の資料をもとにしています。
          それぞれの画面に出典へのリンクがあります。数字や引用は、必ず原本で確かめてください。
        </p>
      </Section>

      <Section title="運営">
        <List>
          <li>
            運営者: <ToFill>運営者の名前（個人名または屋号）</ToFill>
          </li>
          <li>
            収支の公開: <ToFill>収支を公開するページの URL</ToFill>
          </li>
        </List>
      </Section>

      <Section title="ソースコード">
        <p>
          このサービスのソースコードは、GNU Affero General Public License v3
          （AGPL-3.0-only）で公開しています。動いているコードは、公開しているものと同じです。
          {sourceUrl && (
            <>
              {" "}
              <a
                href={sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="underline underline-offset-2"
              >
                ソースコードを見る
              </a>
            </>
          )}
        </p>
      </Section>

      <Section id="contact" title="お問い合わせ">
        <p>
          次のようなご連絡は、メールでお送りください: <ContactEmail />
        </p>
        <List>
          <li>
            引用や数字の誤りの訂正（議員・自治体の方からのご連絡も含みます）
          </li>
          <li>自分が出した意見の削除の依頼</li>
          <li>不適切な意見の報告</li>
          <li>不具合の報告</li>
        </List>
        <p>
          個人で運営しているため、お返事に時間がかかることがあります。
          使い方の決まりは
          <Link href="/terms" className="underline underline-offset-2">
            利用規約
          </Link>
          、集める情報は
          <Link href="/privacy" className="underline underline-offset-2">
            プライバシーポリシー
          </Link>
          をご覧ください。
        </p>
      </Section>
    </InfoPage>
  );
}
