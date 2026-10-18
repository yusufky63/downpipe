// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "{site} 播放列表",
    channel: "{site} 频道",
  },
  untitled: "播放列表",
  reading: "正在向 yt-dlp 获取视频…",
  unavailable: "此播放列表或频道不可用：它是私享内容、已被移除或在你所在的地区被屏蔽。",
  empty: "yt-dlp 在此列表中没有找到视频。",
  truncated: { other: "仅列出前 {count} 个视频。" },
  filter: "按标题筛选",
  selectAll: "全选",
  selectNone: "全不选",
  selected: { other: "已选择 {count} / {total}" },
  rowsLabel: "视频",
  notAvailable: "不可用",
  untitledVideo: "无标题视频",
  quality: "画质",
  qualityHint: "所有视频使用同一画质；没有该画质的视频会使用低一级的画质。",
  presets: {
    best: "最佳",
    "1080p": "最高 1080p",
    "720p": "最高 720p",
    audio: "仅音频",
  },
  autoCaptions: "没有字幕时使用自动字幕",
  noFfmpeg: "未找到 FFmpeg：只能下载自带声音的画质，在 YouTube 上最高为 360p。",
  folder: "文件夹",
  folderHint: "在旁边的文件夹中创建。留空则不单独建文件夹。",
  saveIn: "保存到",
  submit: { other: "下载 {count} 个视频" },
  addToList: { other: "添加 {count} 个视频" },
  added: { other: "已添加 {count} 个视频。" },
  skipped: { other: "{count} 个已在列表中。" },
};
