// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  audioOnly: "仅音频",
  defaultQuality: "默认",
  aboutSize: "约 {size}",
  audio: "音频",
  tools: {
    found: "已找到",
    notFound: "未找到",
    problem: "有问题",
    version: "版本 {version}",
    sources: {
      settings: "在设置中指定",
      managed: "由 Downpipe 下载",
      path: "位于 PATH 中",
    },
    missing: {
      ytDlp: "下载页面中的视频需要它。",
      runtime: "没有 Deno（或 Node），yt-dlp 会缺少部分 YouTube 画质。",
      ffmpeg: "没有它，只有“仅音频”和自带声音的画质可用。",
    },
    download: "下载 {tools}",
    update: "更新 {tools}",
  },
};
