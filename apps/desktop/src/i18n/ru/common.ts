// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Отмена",
  save: "Сохранить",
  close: "Закрыть",
  browse: "Обзор",
  dismiss: "Скрыть",
  keep: "Оставить",
  open: "Открыть",
  pause: "Приостановить",
  resume: "Возобновить",
  retry: "Повторить",
  remove: "Удалить",
  showInFolder: "Показать в папке",
  moreActions: "Другие действия",
  unknown: "Неизвестно",
  none: "–",
  siteVideo: "Видео {site}",
  downloads: { one: "{count} загрузка", few: "{count} загрузки", many: "{count} загрузок", other: "{count} загрузки" },
  links: { one: "{count} ссылка", few: "{count} ссылки", many: "{count} ссылок", other: "{count} ссылки" },
};

export const states: Messages["states"] = {
  confirm: "Подтверждение",
  queued: "В очереди",
  starting: "Подключение",
  downloading: "Загрузка",
  waiting: "Повтор",
  verifying: "Проверка",
  paused: "Приостановлено",
  completed: "Готово",
  failed: "Ошибка",
  canceled: "Отменено",
  filters: {
    all: "Все",
    active: "Активные",
    queued: "В очереди",
    completed: "Завершенные",
    failed: "С ошибками",
  },
};
