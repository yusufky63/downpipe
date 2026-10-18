// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "新建下载",
  title: "添加链接",
  titleMany: { other: "添加 {count} 个链接" },
  addPaused: "以暂停状态添加",
  submit: {
    next: "下一步",
    addToList: "添加到列表",
    download: "下载",
    downloadMany: { other: "下载 {count} 个文件" },
  },
  links: {
    label: "链接",
    hint: "每行一个链接，或使用 file[001-100].zip 这样的模式。支持 http、https 和 magnet 链接。",
    found: { other: "找到 {count} 个链接，每个链接各一个下载。" },
    fromPattern: { other: "从模式生成 {count} 个链接，每个链接各一个下载。" },
    patternCapped: { other: "仅使用模式中的前 {count} 个链接。" },
    video: "一个 {site} 视频。接下来选择其画质。",
    allVideos: "接下来选择每个视频的画质。",
    someVideos: { other: "其中 {count} 个是视频，接下来选择其画质。" },
    importList: "导入列表",
    importHint: "每行一个链接的 .txt 文件。也可以将其拖放到这里。",
  },
  saveTo: {
    label: "保存到",
    automatic: "自动：文件所属类别的文件夹。",
  },
  fileName: {
    label: "文件名",
    placeholder: "自动",
    hint: "留空则使用服务器提供的名称。",
    videoHint: "留空则使用视频标题。",
    singleOnly: "仅适用于单个链接。",
  },
  connections: {
    label: "连接数",
    hint: "上限",
    videoHint: "不适用于视频",
  },
  queue: {
    label: "队列",
    hint: "每个队列各自同时运行自己的下载。",
  },
  checksum: {
    label: "校验和",
    hint: "可选，例如 sha256:9f86d0… 仅当校验和匹配时，文件才会被重命名到最终位置。",
    videoHint: "仅适用于文件：yt-dlp 会用网站提供的格式合成视频文件。",
  },
  errors: {
    listTooLarge: "该文件太大，不能作为链接列表。",
    noLinks: "请至少粘贴一个 http、https 或 magnet 链接。",
    forLink: "{url}：{error}",
  },
};
