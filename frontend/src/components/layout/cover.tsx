// SPDX-License-Identifier: AGPL-3.0-only

import { SITE_TAGLINE } from "@/lib/metadata";

/**
 * その日に初めて開いたときだけ、ロゴと一言を画面いっぱいに出すカバー。タップ（かキー）で閉じる。
 * 中身は下で普通に読み込む（重ねるだけ。閉じると開いたページがそのまま見える）。
 * 出す・閉じるは COVER_SCRIPT が <html data-cover> で決める（React より先に動くので、読み込みの途中でも閉じられる）。
 * 見た目は globals.css の .cover
 */
export function Cover() {
  return (
    // 読み上げでは飛ばす（中身は下にそのまま読める）。キーでも閉じられる（COVER_SCRIPT）
    <div className="cover" aria-hidden="true">
      {/* ロゴは CSS の背景にする（出さない日に取りに行かない） */}
      <div className="cover-logo" />
      {/* 狭い画面では「、」のあとで折り返す */}
      <p className="cover-tagline">
        {SITE_TAGLINE.split(/(?<=、)/).map((part) => (
          <span key={part} className="inline-block">
            {part}
          </span>
        ))}
      </p>
      <p className="cover-hint">タップしてはじめる</p>
    </div>
  );
}

/** 端末に残す印（localStorage。最後に出した日）。プライバシーポリシーに書いてある */
const SEEN_KEY = "cp_cover_seen";

/**
 * <head> で同期に動かす。
 * - 今日（端末の日付）もう出していれば data-cover="seen"（出さない）。まだなら今日の日付を残す。
 *   localStorage を使えないとき（ブラウザの設定など）は、毎回出さないように出さない
 * - 出ているあいだ、カバーのタップか、どれかのキーで data-cover="closing"（薄くなって消える）
 */
export const COVER_SCRIPT = `(function(){var d=document.documentElement;try{var n=new Date(),t=n.getFullYear()+"-"+(n.getMonth()+1)+"-"+n.getDate();if(localStorage.getItem("${SEEN_KEY}")===t){d.setAttribute("data-cover","seen");return}localStorage.setItem("${SEEN_KEY}",t)}catch(e){d.setAttribute("data-cover","seen");return}function c(){if(!d.hasAttribute("data-cover"))d.setAttribute("data-cover","closing")}document.addEventListener("click",function(e){if(e.target.closest&&e.target.closest(".cover"))c()},true);document.addEventListener("keydown",c,true)})()`;

/** JS が動かないとき（COVER_SCRIPT が無い）は、閉じられないので少し待って消す */
export const COVER_NOSCRIPT_STYLE =
  "html .cover{animation:cover-out 400ms ease-in 2500ms forwards}";
