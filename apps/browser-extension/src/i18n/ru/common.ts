// Words used on several screens. Product names (Downpipe), formats (HLS, MP4)
// and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Загрузить",
  sending: "Отправка…",
  checkAgain: "Проверить снова",
  setupGuide: "Руководство по настройке",
  filters: "Фильтры",
  unit: "Единица",
  extensionNoAnswer: "Расширение не ответило.",
  justNow: "только что",
  // Abbreviated units take no agreement with the number.
  minutesAgo: { one: "{count} мин назад", few: "{count} мин назад", many: "{count} мин назад", other: "{count} мин назад" },
  hoursAgo: { one: "{count} ч назад", few: "{count} ч назад", many: "{count} ч назад", other: "{count} ч назад" },
};

export const results: Messages["results"] = {
  added: "Добавлено в Downpipe",
  waiting: "Ожидает вашего ответа в Downpipe",
  sent: "Отправлено в Downpipe",
};
