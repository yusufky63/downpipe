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
    hint: "每行一个链接，或使用 file[001-100].zip 这样的模式。仅支持 http 和 https。",
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
  /** Sent with every request, kept in memory only. */
  headers: {
    label: "请求头",
    hint: "可选。每行一个，格式为 名称: 值。随每个请求发送，只保存在内存中，从不写入磁盘。",
  },
  /** "Paste as cURL": a command from the browser's developer tools. */
  curl: {
    button: "以 cURL 粘贴",
    label: "cURL 命令",
    hint: "在浏览器的开发者工具中：网络，右键点击请求，复制为 cURL。只能下载 GET 请求。",
    apply: "填入",
    filled: "已从命令填入：链接及其请求头。",
    ignored: "已略过：{list}。",
    notCurl: "这不是 cURL 命令。它必须以 curl 开头。",
    quote: "命令中有一个引号没有闭合。请重新复制完整的命令。",
    noUrl: "命令中没有 http 或 https 链接。",
    method: "此请求使用 {method}。Downpipe 只用 GET 下载，因此无法重复此请求。",
    data: "此请求会发送数据（{option}），即为 POST 请求。Downpipe 只用 GET 下载。",
  },
  errors: {
    listTooLarge: "该文件太大，不能作为链接列表。",
    noLinks: "请至少粘贴一个 http 或 https 链接。",
    forLink: "{url}：{error}",
    headerFormat: "第 {line} 行：请写成 名称: 值 的格式。",
    headerName: "第 {line} 行：{name} 不是有效的请求头名称。",
    headerManaged: "第 {line} 行：{name} 由 Downpipe 自行设置。",
  },
};
