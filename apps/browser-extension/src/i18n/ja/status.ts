// The connection to the desktop app. The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "接続を確認中",
  connected: "Downpipe に接続済み",
  connectedDetail: "アプリ {app}、ブラウザーブリッジ {bridge}",
  appNotRunning: "Downpipe が実行されていません",
  appNotRunningDetail: "Downpipe アプリを開いてください。アプリが閉じている間に送信したリンクはキューに入りません。",
  hostMissing: "ブラウザーブリッジがインストールされていません",
  hostMissingDetail:
    "Downpipe アプリを一度開いてください。起動時にブリッジが登録されます。ソースからビルドした場合は、次を実行します：",
  hostForbidden: "ブリッジがこの拡張機能を許可していません",
  hostForbiddenDetail: "この拡張機能の ID 用にブリッジを登録してから、ブラウザーを再起動してください：",
  error: "ブリッジのエラー",
};
