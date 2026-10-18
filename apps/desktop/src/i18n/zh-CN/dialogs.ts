// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "移除并删除文件",
    deleteFiles: "移除并删除文件",
    deletePartial: "移除并删除未完成的数据",
    fromList: "从列表中移除",
    doneOne: "从列表中移除后，文件仍保留在磁盘上。删除则会同时删除文件本身。",
    doneMany: "从列表中移除后，这些文件仍保留在磁盘上。删除则会同时删除文件本身。",
    partialOne: "从列表中移除会保留未完成的文件，以后仍可通过同一链接继续下载。删除则会同时删除未完成的数据。",
    partialMany: "从列表中移除会将已完成的文件和未完成的数据保留在磁盘上。删除则会将两者一并删除。",
  },
  redownload: {
    eyebrow: "重新下载",
    body: "从同一链接将文件重新下载到同一文件夹，例如当服务器上的文件已更改或磁盘上的副本已损坏时。",
    replace: "替换现有文件",
    replaceOn: "先删除当前文件，新文件沿用其名称。",
    replaceOff: "关闭：保留当前文件，新副本以一个未被占用的名称保存在其旁边。",
    confirm: "重新下载",
    confirmReplace: "删除并重新下载",
  },
  duplicate: {
    eyebrow: "已在列表中",
    titleMany: { other: "{count} 个链接已在列表中" },
    body: "列表中已有使用此地址的下载（{state}）。要怎么做？",
    bodyMany: {
      other: "列表中已有使用这些地址的下载。要如何处理这 {count} 个链接？",
    },
    question: "如何处理该链接",
    copy: "再次添加，并使用带编号的文件名",
    overwrite: "再次添加，并替换现有文件",
    existing: "如果现有下载已完成则显示它，否则继续下载",
    ok: "确定",
    remember: "不再询问，始终这样做",
    rememberHint: "可在“设置”的“下载”中改回。",
  },
  /** A download's own speed limit, under the global one. {limit} is like "2 MiB/s". */
  speedLimit: {
    eyebrow: "速度限制",
    body: "仅用于此下载，且不超过全局限制 {limit}。",
    bodyNoGlobal: "仅用于此下载。当前没有全局限制。",
    bodyMany: "分别用于每个下载，且不超过全局限制 {limit}。",
    bodyManyNoGlobal: "分别用于每个下载。当前没有全局限制。",
    label: "限制",
    unit: "速度限制单位",
    placeholder: "不限速",
    video: "视频在下次开始时才使用新的限制。",
    remove: "取消限制",
    save: "设置限制",
  },
  /** "Pause until…" with a time of the user's choice. */
  pauseUntil: {
    eyebrow: "暂停到",
    label: "继续时间",
    hint: "现在暂停，Downpipe 运行时会在此时间自动继续；若当时未运行，则在之后启动时继续。",
    past: "请选择一个将来的时间。",
    invalid: "请输入日期和时间。",
    submit: "暂停",
  },
};
