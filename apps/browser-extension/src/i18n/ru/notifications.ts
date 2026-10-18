// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Ожидает вашего ответа в Downpipe",
  waitingBody: "Подтвердите имя и папку в Downpipe, чтобы начать загрузку.",
  downloading: "Загружается в Downpipe",
  duplicate: "Уже есть в Downpipe",
  kept: "Загружено браузером",
  failed: "Не отправлено в Downpipe",
  toolbarTitle: "Downpipe: {label}. {message}",
};
