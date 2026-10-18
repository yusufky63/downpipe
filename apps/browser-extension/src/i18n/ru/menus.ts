// Right-click menu items. popup.emptyActivity, links.gone and the desktop
// app's settings quote them; keep them the same.
import type { Messages } from "../core";

export const menus: Messages["menus"] = {
  list: "Выбрать видео из этого списка с помощью Downpipe…",
  link: "Загрузить с помощью Downpipe",
  torrent: "Загрузить торрент с помощью Downpipe",
  pageLinks: "Загрузить ссылки на этой странице с помощью Downpipe…",
  video: "Загрузить это видео с помощью Downpipe",
  selection: "Загрузить выбранные ссылки с помощью Downpipe…",
};
