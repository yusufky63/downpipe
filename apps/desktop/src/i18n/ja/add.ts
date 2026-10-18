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
    hint: "1 行に 1 つのリンク、または file[001-100].zip のようなパターンを入力します。http、https、magnet のリンクに対応しています。",
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
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "リクエストヘッダー",
    hint: "省略可。1 行に 1 つ、名前: 値 の形式で入力します。すべてのリクエストで送信され、メモリーにのみ保持され、保存されることはありません。",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "cURL として貼り付け",
    label: "cURL コマンド",
    hint: "ブラウザーの開発者ツールで「ネットワーク」を開き、リクエストを右クリックして「cURL としてコピー」を選びます。ダウンロードできるのは GET リクエストだけです。",
    apply: "入力",
    filled: "コマンドからリンクとそのヘッダーを入力しました。",
    ignored: "除外した項目: {list}。",
    notCurl: "これは cURL コマンドではありません。curl で始まる必要があります。",
    quote: "コマンド内の引用符が閉じていません。コマンド全体をもう一度コピーしてください。",
    noUrl: "コマンドに http または https のリンクがありません。",
    method: "このリクエストは {method} を使っています。Downpipe は GET でしかダウンロードしないため、再現できません。",
    data: "このリクエストはデータを送信する（{option}）ため POST になります。Downpipe は GET でしかダウンロードしません。",
  },
  errors: {
    listTooLarge: "このファイルはリンクのリストとしては大きすぎます。",
    noLinks: "http、https、magnet のいずれかのリンクを 1 つ以上貼り付けてください。",
    forLink: "{url}: {error}",
    headerFormat: "{line} 行目: 名前: 値 の形式で入力してください。",
    headerName: "{line} 行目: {name} は有効なヘッダー名ではありません。",
    headerManaged: "{line} 行目: {name} は Downpipe が自分で設定します。",
  },
};
