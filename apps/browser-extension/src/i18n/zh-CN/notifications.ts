// System notifications after a hand-over, and the toolbar button's tooltip.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "正在 Downpipe 中等待你的确认",
  waitingBody: "在 Downpipe 中确认其名称和文件夹即可开始下载。",
  downloading: "正在 Downpipe 中下载",
  duplicate: "已在 Downpipe 中",
  kept: "已由浏览器下载",
  failed: "未发送到 Downpipe",
  toolbarTitle: "Downpipe：{label}。{message}",
};
