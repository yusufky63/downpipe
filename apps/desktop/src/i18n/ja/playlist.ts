// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "{site} の再生リスト",
    channel: "{site} のチャンネル",
  },
  untitled: "再生リスト",
  reading: "yt-dlp に動画を問い合わせています…",
  unavailable: "この再生リストまたはチャンネルは利用できません。非公開、削除済み、またはお住まいの地域でブロックされています。",
  empty: "yt-dlp はこのリストに動画を見つけられませんでした。",
  truncated: { other: "最初の {count} 本だけを表示しています。" },
  filter: "タイトルで絞り込む",
  selectAll: "すべて選択",
  selectNone: "選択を解除",
  selected: { other: "{total} 本中 {count} 本を選択" },
  rowsLabel: "動画",
  notAvailable: "利用不可",
  untitledVideo: "タイトルのない動画",
  quality: "画質",
  qualityHint: "すべての動画に同じ画質を使います。その画質がない動画には、次に低い画質を使います。",
  presets: {
    best: "最高画質",
    "1080p": "最大 1080p",
    "720p": "最大 720p",
    audio: "音声のみ",
  },
  autoCaptions: "字幕がない動画では自動字幕を使う",
  noFfmpeg: "FFmpeg が見つかりません。音声付きの画質しかダウンロードできず、YouTube では最大 360p になります。",
  folder: "フォルダー",
  folderHint: "隣のフォルダーの中に作成します。空欄の場合は専用のフォルダーを作りません。",
  saveIn: "保存先",
  submit: { other: "{count} 本の動画をダウンロード" },
  addToList: { other: "{count} 本の動画を追加" },
  added: { other: "{count} 本の動画を追加しました。" },
  skipped: { other: "{count} 本は既にリストにあります。" },
};
