// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "取消",
  save: "保存",
  close: "关闭",
  browse: "浏览",
  dismiss: "关闭",
  keep: "保留",
  open: "打开",
  pause: "暂停",
  resume: "继续",
  retry: "重试",
  remove: "移除",
  showInFolder: "在文件夹中显示",
  moreActions: "更多操作",
  unknown: "未知",
  none: "–",
  siteVideo: "{site} 视频",
  downloads: { other: "{count} 个下载" },
  links: { other: "{count} 个链接" },
};

export const states: Messages["states"] = {
  confirm: "待确认",
  queued: "排队中",
  starting: "连接中",
  downloading: "下载中",
  waiting: "重试中",
  verifying: "校验中",
  paused: "已暂停",
  completed: "已完成",
  failed: "失败",
  canceled: "已取消",
  filters: {
    all: "全部",
    active: "进行中",
    queued: "排队中",
    completed: "已完成",
    failed: "失败",
  },
};
