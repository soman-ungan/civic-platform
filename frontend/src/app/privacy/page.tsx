// SPDX-License-Identifier: AGPL-3.0-only

import type { Metadata } from "next";
import Link from "next/link";

import {
  ContactEmail,
  InfoPage,
  List,
  Section,
  SiteValue,
} from "@/components/info/prose";
import { pageMetadata } from "@/lib/metadata";

export const metadata: Metadata = pageMetadata(
  "プライバシーポリシー",
  "集める情報・集めない情報と、その使い方・守り方です。",
);

export default function PrivacyPage() {
  return (
    <InfoPage title="プライバシーポリシー" updated="2026年10月31日">
      <p className="mt-3">
        本サービスは、参加する方の情報をできるだけ集めない作りにしています。
        名前・メールアドレス・電話番号・年齢・性別・住んでいる地区は集めません。
        このページでは、集めるもの・使い方・守り方を説明します。
      </p>

      <Section title="1. 集める情報">
        <List>
          <li>
            <strong>参加者の ID</strong>:
            初めて投票などをしたときに作る、意味の無い乱数です。 ブラウザの
            Cookie（<code>cp_uid</code>。改ざんを防ぐ署名つき、1年）に入れ、
            同じ人の投票をまとめるのに使います
          </li>
          <li>
            <strong>そのまちとの関わり方</strong>:
            住民・周辺に住む・通勤通学・関係人口のどれか（自治体ごと）
          </li>
          <li>
            <strong>投票・意見・トピックの提案</strong>: 参加者の ID
            とひも付けて保存します
          </li>
          <li>
            <strong>パスキー</strong>（作った方だけ）: 公開鍵と鍵の
            ID、作った日・最後に使った日です。 指紋・顔・PIN
            は端末の中だけで使われ、本サービスには届きません
          </li>
          <li>
            <strong>サーバーのアクセスの記録</strong>: IP
            アドレス・日時・ブラウザの種類など。
            障害の対応と不正な利用を防ぐためだけに使い、
            <SiteValue field="accessLogRetention" />
            で消します
          </li>
        </List>
        <p>
          ほかに、ブラウザの中（localStorage・sessionStorage）に「この端末でパスキーを作ったか」「ログインせずに投票すると選んだか」「最初の画面を最後に出した日」の印だけを置きます。
          広告や、利用状況を外部の会社に送る解析のツールは使っていません。
        </p>
      </Section>

      <Section title="2. 使い方">
        <List>
          <li>投票と意見から、意見のグループと合意点を計算して見せるため</li>
          <li>1人が出せる意見の数などの決まりを守るため</li>
          <li>不適切な投稿や不正な利用を防ぐため</li>
        </List>
        <p>
          誰が何に投票したか、どの意見を書いたかは、運営者の画面も含めてどこにも出しません。
        </p>
      </Section>

      <Section title="3. 公開する情報">
        <p>
          公開するのは、意見の本文と、投票を集計した数字だけです。関わり方ごとの集計は、
          5人未満の区分を隠し、引き算で分からないよう、もう1つの区分も隠すことがあります。
          人を年齢・性別・地区で分けた集計はしません。
        </p>
      </Section>

      <Section title="4. AI に送る情報">
        <p>
          意見を公開してよいかの確認とトピックの分類に、Anthropic 社（米国）の
          AI（Claude）を使います。
          送るのは、投稿された意見の本文と、公開しているトピックの内容（タイトル・説明・公開済みの意見）だけです。
          参加者の ID や関わり方など、あなたを見分ける情報は送りません。
        </p>
        <p>
          意見の本文に個人の情報を書かないでください。送った内容の扱いは、Anthropic
          社の規約に従います （
          <SiteValue field="aiDataHandling" />
          ）。
        </p>
      </Section>

      <Section title="5. 情報を渡す先">
        <p>
          法令に基づく場合を除き、集めた情報を第三者に渡しません。本サービスを動かすサーバーは
          <SiteValue field="hosting" />
          にあります。データを失わないためのバックアップは
          <SiteValue field="backup" />
          に暗号化した通信で送って保存します。
        </p>
      </Section>

      <Section title="6. 守り方">
        <p>
          通信は暗号化（HTTPS）します。Cookie
          は署名し、ブラウザのスクリプトからは読めない設定にしています。
          パスキーの仕組みでは、本サービスに漏れて困るパスワードを持ちません。
        </p>
      </Section>

      <Section title="7. アカウントの削除">
        <p>
          パスキーを作った方は、
          <Link href="/account" className="underline underline-offset-2">
            アカウントのページ
          </Link>
          から、いつでもアカウントを削除できます。パスキー・関わり方・登録の印を消し、Cookie
          も消します。
          それまでの投票と意見は、誰のものか分からないまま残ります（分析の結果が変わらないようにするため）。
          意見そのものを消したいとき、パスキーを作っていない方が情報の削除を求めるときは、問い合わせからご連絡ください。
        </p>
        <p>
          消した情報も、バックアップには最長で
          <SiteValue field="backupRetention" />
          残り、そのあと自動で消えます。
        </p>
        <p>
          ブラウザの Cookie を消すと、その端末の参加者の ID
          は使えなくなります（パスキーを作っていれば、ログインで戻れます）。
        </p>
      </Section>

      <Section title="8. このポリシーの変更">
        <p>
          このポリシーは変えることがあります。変えるときは、このページで知らせます。
        </p>
      </Section>

      <Section title="お問い合わせ">
        <p>
          <ContactEmail />
        </p>
      </Section>
    </InfoPage>
  );
}
