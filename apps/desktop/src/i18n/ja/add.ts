// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "新しいダウンロード",
  title: "リンクを追加",
  titleMany: { other: "{count} 件のリンクを追加" },
  addPaused: "一時停止状態で追加",
  submit: {
    next: "次へ",
    addToList: "一覧に追加",
    download: "ダウンロード",
    downloadMany: { other: "{count} 個のファイルをダウンロード" },
  },
  links: {
    label: "リンク",
    hint: "1 行に 1 つのリンク、または file[001-100].zip のようなパターンを入力します。http と https のみ対応しています。",
    found: { other: "{count} 件のリンクが見つかりました。リンクごとに 1 つのダウンロードになります。" },
    fromPattern: { other: "パターンから {count} 件のリンクを作成しました。リンクごとに 1 つのダウンロードになります。" },
    patternCapped: { other: "パターンの最初の {count} 件のリンクのみ使用されます。" },
    video: "{site} の動画です。次に画質を選択します。",
    allVideos: "次に各動画の画質を選択します。",
    someVideos: { other: "そのうち {count} 件は動画です。次に画質を選択します。" },
    importList: "リストをインポート",
    importHint: "1 行に 1 つのリンクを書いた .txt ファイルです。ここにドロップすることもできます。",
  },
  saveTo: {
    label: "保存先",
    automatic: "自動：ファイルのカテゴリのフォルダーに保存します。",
  },
  fileName: {
    label: "ファイル名",
    placeholder: "自動",
    hint: "空欄にすると、サーバーが送る名前を使用します。",
    videoHint: "空欄にすると、動画のタイトルを使用します。",
    singleOnly: "リンクが 1 件のときのみ指定できます。",
  },
  connections: {
    label: "接続数",
    hint: "上限",
    videoHint: "動画には適用されません",
  },
  queue: {
    label: "キュー",
    hint: "キューごとに、それぞれのダウンロードを同時に実行します。",
  },
  checksum: {
    label: "チェックサム",
    hint: "省略可。例：sha256:9f86d0… 一致した場合にのみ、ファイルが本来の名前に変更されます。",
    videoHint: "ファイルのみ対象です。動画のファイルは、yt-dlp がサイトのフォーマットを組み合わせて作成します。",
  },
  errors: {
    listTooLarge: "このファイルはリンクのリストとしては大きすぎます。",
    noLinks: "http または https のリンクを 1 つ以上貼り付けてください。",
    forLink: "{url}: {error}",
  },
};
