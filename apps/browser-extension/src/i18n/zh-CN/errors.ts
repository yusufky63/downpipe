// Problems the extension words itself.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "未安装 Downpipe 浏览器桥接。",
    host_forbidden: "浏览器桥接已安装，但不允许此扩展连接。",
    host_error: "浏览器桥接意外停止。",
    timeout: "Downpipe 未及时响应。",
    bad_response: "浏览器桥接发送了此扩展无法识别的响应。",
  },
  code: {
    app_not_running: "Downpipe 未运行。请打开 Downpipe 应用后重试。",
    app_unavailable: "Downpipe 没有响应。请重新启动应用后重试。",
    duplicate: "已在 Downpipe 中。",
    invalid_url: "无法下载此链接。仅支持 http 和 https 链接。",
    version_mismatch: "Downpipe 与其浏览器桥接的版本不同。请重新启动 Downpipe，让它注册自己的桥接。",
    unsupported_version: "此扩展比已安装的 Downpipe 更新。请更新 Downpipe。",
    message_too_large: "链接太长，无法传递。",
    unsupported_page: "Downpipe 不支持下载此网站的视频。",
  },
  refused: "Downpipe 拒绝了此下载（{code}）。",
  streamsUnsupported: "此版本的 Downpipe 无法下载流。请更新 Downpipe。",
  pagesUnsupported: "此版本的 Downpipe 无法下载页面中的视频。请更新 Downpipe。",
  keptClosed: "Downpipe 未运行，因此已由浏览器下载。",
  keptInstead: "{reason} 已改由浏览器下载。",
  notVideoPage: "Downpipe 不支持下载此页面中的视频。",
  unknownVideoPage: "这不是 Downpipe 能识别的视频页面。",
  mediaGone: "此媒体已不在列表中。请重新播放。",
  mediaInvalid: "无法下载此媒体。",
  streamInvalid: "无法下载此流。",
};
