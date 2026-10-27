// SPDX-License-Identifier: AGPL-3.0-only

import { ViewTransition } from "react";

const SLIDE = {
  "nav-forward": "nav-forward",
  "nav-back": "nav-back",
  default: "none",
};

/**
 * タブの切り替えで、ページの本体を左右にスライドさせる（globals.css の .nav-forward / .nav-back）。
 * 種類の無い遷移（ブラウザの戻る・再読み込み）は動かさない。レイアウトではなく各ページに置く
 * （レイアウトは遷移の間も残るので、enter / exit が起きない）
 */
export function PageSlide({ children }: { children: React.ReactNode }) {
  return (
    <ViewTransition enter={SLIDE} exit={SLIDE} default="none">
      {children}
    </ViewTransition>
  );
}
