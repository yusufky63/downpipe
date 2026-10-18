// Words used on several screens. Product names, formats and units stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "下载",
  sending: "正在发送…",
  checkAgain: "重新检查",
  setupGuide: "设置指南",
  filters: "筛选器",
  unit: "单位",
  extensionNoAnswer: "扩展没有响应。",
  justNow: "刚刚",
  minutesAgo: { other: "{count} 分钟前" },
  hoursAgo: { other: "{count} 小时前" },
};

export const results: Messages["results"] = {
  added: "已添加到 Downpipe",
  waiting: "正在 Downpipe 中等待你的确认",
  sent: "已发送到 Downpipe",
};
