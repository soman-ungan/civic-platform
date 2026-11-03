// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 初めて開いたときだけ、ロゴを画面いっぱいに出してから消すカバー。
 * 中身は下で普通に読み込む（重ねるだけ）。消えるのは CSS のアニメーションなので JS が無くても消える。
 * 2回目からは、描く前に COVER_SCRIPT が <html data-cover="seen"> を付けて出さない（ちらつかない）。
 * 見た目は globals.css の .cover
 */
export function Cover() {
  // 画像は CSS の背景にする（2回目から display:none のとき、取りに行かない）
  return <div className="cover" aria-hidden="true" />;
}

/** 端末に残す印（localStorage）。プライバシーポリシーに書いてある */
const SEEN_KEY = "cp_cover_seen";

/**
 * <head> で同期に動かす。見たことがあれば出さず、無ければ印を付ける。
 * localStorage を使えないとき（ブラウザの設定など）は、毎回出さないように出さない
 */
export const COVER_SCRIPT = `(function(){var d=document.documentElement;try{if(localStorage.getItem("${SEEN_KEY}")){d.setAttribute("data-cover","seen")}else{localStorage.setItem("${SEEN_KEY}","1")}}catch(e){d.setAttribute("data-cover","seen")}})()`;
