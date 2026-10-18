// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "キャンセル",
  save: "保存",
  close: "閉じる",
  browse: "参照",
  dismiss: "閉じる",
  keep: "残す",
  open: "開く",
  pause: "一時停止",
  resume: "再開",
  retry: "再試行",
  remove: "削除",
  showInFolder: "フォルダーに表示",
  moreActions: "その他の操作",
  unknown: "不明",
  none: "–",
  siteVideo: "{site} 動画",
  downloads: { other: "{count} 件のダウンロード" },
  links: { other: "{count} 件のリンク" },
};

export const states: Messages["states"] = {
  confirm: "確認待ち",
  queued: "待機中",
  starting: "接続中",
  downloading: "ダウンロード中",
  waiting: "再試行中",
  verifying: "検証中",
  paused: "一時停止中",
  completed: "完了",
  failed: "失敗",
  canceled: "キャンセル済み",
  why: {
    noData: "サーバーからデータが届きません。再接続しています",
    network: "ネットワークに接続できません。復旧したら再試行します",
    throttled: "サーバーから速度を落とすよう求められました。待機しています",
    server: "サーバーエラー。再試行しています",
  },
  filters: {
    all: "すべて",
    active: "進行中",
    queued: "待機中",
    completed: "完了",
    failed: "失敗",
  },
};
