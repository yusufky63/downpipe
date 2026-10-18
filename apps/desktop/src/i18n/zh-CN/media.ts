// Stream and video page qualities, their audio tracks, and the site tools
// (yt-dlp, Deno or Node, FFmpeg) in the settings.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  subtitles: {
    label: "字幕",
    none: "无",
    uploaded: "上传者提供",
    auto: "自动字幕",
    original: "{language}（原始）",
    embedded: "字幕会嵌入视频文件。",
    separate: "字幕会作为单独的文件保存在视频旁边。",
  },
  siteErrors: {
    forbidden: "YouTube 拒绝了此次下载。更新 yt-dlp 通常可以解决：请在“设置”的“媒体”中更新，然后重试。",
    signIn: "YouTube 要求使用已登录的浏览器进行此次下载。Downpipe 不使用浏览器 Cookie：请稍等片刻或换一个网络，然后重试。",
    ageRestricted: "YouTube 只向已登录的成年用户显示此视频。Downpipe 不使用账号或浏览器 Cookie，因此无法下载。",
    unavailable: "此视频不可用：它是私享视频、已被移除、仅限会员观看或在你所在的地区被屏蔽。",
    jsRuntime: "yt-dlp 无法应答 YouTube 的 JavaScript 验证。请在“设置”的“媒体”中下载 Deno，然后重试。",
    poToken: "YouTube 要求为此视频提供来源证明（PO）令牌。请在“设置”的“媒体”中更新 yt-dlp，或选择其他画质。",
    outdated: "yt-dlp 已无法解析此页面，YouTube 可能更改了页面。请在“设置”的“媒体”中更新 yt-dlp，然后重试。",
  },
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
