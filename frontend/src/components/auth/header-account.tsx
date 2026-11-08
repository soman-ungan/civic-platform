// SPDX-License-Identifier: AGPL-3.0-only

import { AccountLinks } from "@/components/auth/account-links";
import { OperatorLink } from "@/components/auth/operator-link";

/**
 * ヘッダーの右上: 運営者には「管理画面」、だれにでも「パスキーでログイン」か「ログイン中・ログアウト」。
 * 読むことと投票はログインなしでできるので、小さく出す
 */
export function HeaderAccount() {
  return (
    <div className="flex shrink-0 flex-wrap items-center justify-end gap-x-3 gap-y-1 text-xs text-zinc-600 dark:text-zinc-400">
      <OperatorLink />
      <AccountLinks />
    </div>
  );
}
