// Videos and audio detected on pages. {site} is a site's own name, {format} a file format.
import type { Messages } from "../core";

export const media: Messages["media"] = {
  sitePlaylist: "{site} 播放列表 · 在 Downpipe 中选择视频",
  siteChannel: "{site} 频道 · 在 Downpipe 中选择视频",
  thisSitePlaylist: "此 {site} 播放列表",
  thisSiteChannel: "此 {site} 频道",
  updateForLists: "更新 Downpipe 以下载播放列表",
  hls: "HLS 流",
  dash: "DASH 流",
  hlsQualities: { other: "HLS · {count} 种画质" },
  fileAudio: "{format} 音频",
  fileVideo: "{format} 视频",
  audio: "音频",
  video: "视频",
  siteVideo: "{site} 视频",
  thisSiteVideo: "此 {site} 视频",
  siteVideoQuality: "{site} 视频 · 在 Downpipe 中选择画质",
  updateForStreams: "更新 Downpipe 以下载流",
  updateForPages: "更新 Downpipe 以下载页面中的视频",
  emptyOff: "开启后会列出页面播放的视频和音频。浏览器会先请求访问所有网站的权限。",
  emptyExcluded: "此处不列出：{site} 是“筛选器”中的排除网站。",
  emptyNothing: "暂无内容。请播放视频；如果视频已在播放，请重新加载页面。",
};
