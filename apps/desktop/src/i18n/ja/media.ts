// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "音声のみ",
  defaultQuality: "既定",
  aboutSize: "約 {size}",
  audio: "音声",
  tools: {
    found: "検出済み",
    notFound: "未検出",
    problem: "問題あり",
    version: "バージョン {version}",
    sources: {
      settings: "設定で指定",
      managed: "Downpipe がダウンロード",
      path: "PATH 上",
    },
    missing: {
      ytDlp: "ページ内の動画に必要です。",
      runtime: "Deno（または Node）がないと、yt-dlp は YouTube の一部の画質を取得できません。",
      ffmpeg: "これがないと、「音声のみ」と、音声を含む画質しか使用できません。",
    },
    download: "{tools} をダウンロード",
    update: "{tools} を更新",
  },
};
