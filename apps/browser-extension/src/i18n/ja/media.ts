// Videos and audio detected on pages. {site} is a site's own name (YouTube),
// {format} a file format in capitals (MP4).
import type { Messages } from "../core";

export const media: Messages["media"] = {
  sitePlaylist: "{site} の再生リスト · 動画は Downpipe で選択",
  siteChannel: "{site} のチャンネル · 動画は Downpipe で選択",
  thisSitePlaylist: "この {site} の再生リスト",
  thisSiteChannel: "この {site} のチャンネル",
  updateForLists: "再生リストをダウンロードするには Downpipe を更新してください",
  hls: "HLS ストリーム",
  dash: "DASH ストリーム",
  hlsQualities: { other: "HLS · {count} 種類の画質" },
  fileAudio: "{format} 音声",
  fileVideo: "{format} 動画",
  audio: "音声",
  video: "動画",
  siteVideo: "{site} 動画",
  thisSiteVideo: "この {site} 動画",
  siteVideoQuality: "{site} 動画 · 画質は Downpipe で選択",
  updateForStreams: "ストリームをダウンロードするには Downpipe を更新してください",
  updateForPages: "ページ内の動画をダウンロードするには Downpipe を更新してください",
  emptyOff:
    "オンにすると、ページで再生される動画と音声が一覧表示されます。最初に、ブラウザーがすべてのサイトへのアクセス許可を求めます。",
  emptyExcluded: "ここには表示されません：{site} は「フィルター」で除外サイトに指定されています。",
  emptyNothing: "まだ何もありません。動画を再生してください。既に再生中だった場合は、ページを再読み込みしてください。",
};
