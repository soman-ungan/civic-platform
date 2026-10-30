// SPDX-License-Identifier: AGPL-3.0-only

/**
 * 合意点のページの枠。`sheet` は、このページから開いたトピック詳細を上に重ねるレイヤ
 * （`@sheet/(..)topics/[id]`。開いていなければ `@sheet/default.tsx` で何も出さない）
 */
export default function ConsensusLayout({
  children,
  sheet,
}: LayoutProps<"/[municipality]/consensus">) {
  return (
    <>
      {children}
      {sheet}
    </>
  );
}
