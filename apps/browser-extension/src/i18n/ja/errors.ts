// Problems the extension words itself.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "Downpipe のブラウザーブリッジがインストールされていません。",
    host_forbidden: "ブラウザーブリッジはインストールされていますが、この拡張機能を許可していません。",
    host_error: "ブラウザーブリッジが予期せず停止しました。",
    timeout: "Downpipe から時間内に応答がありませんでした。",
    bad_response: "ブラウザーブリッジから、この拡張機能が解釈できない応答が返されました。",
  },
  code: {
    app_not_running: "Downpipe が実行されていません。Downpipe アプリを開いてから、もう一度お試しください。",
    app_unavailable: "Downpipe から応答がありませんでした。アプリを再起動してから、もう一度お試しください。",
    duplicate: "既に Downpipe にあります。",
    invalid_url: "このリンクはダウンロードできません。http と https のリンクのみ対応しています。",
    version_mismatch:
      "Downpipe とブラウザーブリッジのバージョンが異なります。Downpipe を再起動して、Downpipe 自身のブリッジを登録させてください。",
    unsupported_version: "この拡張機能は、インストールされている Downpipe より新しいバージョンです。Downpipe を更新してください。",
    message_too_large: "リンクが長すぎるため、渡すことができません。",
    unsupported_page: "Downpipe はこのサイトの動画をダウンロードできません。",
  },
  refused: "Downpipe がダウンロードを拒否しました（{code}）。",
  streamsUnsupported: "このバージョンの Downpipe はストリームをダウンロードできません。Downpipe を更新してください。",
  pagesUnsupported: "このバージョンの Downpipe はページ内の動画をダウンロードできません。Downpipe を更新してください。",
  keptClosed: "Downpipe が実行されていなかったため、ブラウザーでダウンロードしました。",
  keptInstead: "{reason} 代わりにブラウザーでダウンロードしました。",
  notVideoPage: "Downpipe はこのページの動画をダウンロードできません。",
  unknownVideoPage: "Downpipe が認識できる動画ページではありません。",
  mediaGone: "このメディアは一覧から消えました。もう一度再生してください。",
  mediaInvalid: "このメディアはダウンロードできません。",
  streamInvalid: "このストリームはダウンロードできません。",
};
