// System notifications after a hand-over, and the toolbar button's tooltip.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Downpipe で応答を待っています",
  waitingBody: "ダウンロードを開始するには、Downpipe で名前とフォルダーを確認してください。",
  downloading: "Downpipe でダウンロード中",
  duplicate: "既に Downpipe にあります",
  kept: "ブラウザーでダウンロードしました",
  failed: "Downpipe に送信できませんでした",
  toolbarTitle: "Downpipe：{label}。{message}",
};
