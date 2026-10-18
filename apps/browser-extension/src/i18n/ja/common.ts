// Words used on several screens. Product names (Downpipe), formats (HLS, MP4)
// and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "ダウンロード",
  sending: "送信中…",
  checkAgain: "再確認",
  setupGuide: "セットアップガイド",
  filters: "フィルター",
  unit: "単位",
  extensionNoAnswer: "拡張機能から応答がありませんでした。",
  justNow: "たった今",
  minutesAgo: { other: "{count} 分前" },
  hoursAgo: { other: "{count} 時間前" },
};

export const results: Messages["results"] = {
  listOpened: "Downpipe で開きました。動画はそちらで選んでください。",
  added: "Downpipe に追加しました",
  waiting: "Downpipe で応答を待っています",
  sent: "Downpipe に送信しました",
};
