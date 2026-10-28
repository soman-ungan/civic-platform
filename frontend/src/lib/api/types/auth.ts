// SPDX-License-Identifier: AGPL-3.0-only

// ログインの API の型（backend/src/interface/api/auth.rs）

/** ログインの状態（backend/src/interface/api/auth.rs。利用者の ID は返さない） */
export type Me = {
  /** パスキーを登録している（意見を投稿できる） */
  registered: boolean;
  operator: boolean;
};
