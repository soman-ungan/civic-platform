export default function Home() {
  return (
    <main className="mx-auto flex w-full max-w-md flex-1 flex-col px-4 py-8">
      <header className="mb-6">
        <p className="text-sm text-zinc-500">栃木県大田原市</p>
        <h1 className="text-2xl font-bold tracking-tight">
          市民参加プラットフォーム
        </h1>
      </header>
      <section className="rounded-2xl border border-zinc-200 bg-white p-5 shadow-sm dark:border-zinc-800 dark:bg-zinc-900">
        <p className="leading-7 text-zinc-700 dark:text-zinc-300">
          議事録・予算・決算をわかりやすく見せ、みんなの合意点を見つける場所です。トピック一覧はここに並ぶ予定です。
        </p>
      </section>
    </main>
  );
}
