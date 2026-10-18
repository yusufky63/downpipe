// The connection to the desktop app. The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "正在检查连接",
  connected: "已连接到 Downpipe",
  connectedDetail: "应用 {app}，浏览器桥接 {bridge}",
  appNotRunning: "Downpipe 未运行",
  appNotRunningDetail: "请打开 Downpipe 应用。应用关闭时发送的链接不会进入队列。",
  hostMissing: "未安装浏览器桥接",
  hostMissingDetail: "请打开一次 Downpipe 应用，它会在启动时注册桥接。如果是从源代码构建的版本，请运行：",
  hostForbidden: "桥接不允许此扩展",
  hostForbiddenDetail: "为此扩展的 ID 注册桥接，然后重新启动浏览器：",
  error: "桥接错误",
};
