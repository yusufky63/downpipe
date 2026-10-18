// The toolbar popup. The switch labels here are quoted in options.ts and
// links.ts; keep them the same.
import type { Messages } from "../core";

export const popup: Messages["popup"] = {
  connected: "Подключено",
  connectedTitle: "{title}. {detail}",
  desktopApp: "Приложение для компьютера",
  mediaHeading: "Медиа на этой странице",
  detectMedia: "Обнаруживать медиа на страницах",
  downloadsHeading: "Загрузки",
  pageLinks: "Ссылки на этой странице…",
  emptyActivity: "Щелкните ссылку правой кнопкой мыши и выберите «Загрузить с помощью Downpipe».",
  capture: "Перехватывать загрузки браузера",
  captureDetail: "Когда Downpipe закрыт, загрузки остаются в браузере.",
  notifications: "Уведомления",
  notificationsDetail: "Когда загрузка передана в Downpipe или передать ее не удалось.",
  pauseSite: "Не перехватывать на этом сайте",
  resumeSite: "Снова перехватывать на этом сайте",
  pauseSiteTitle: "Загрузки с {host} или начатые на его страницах остаются в браузере",
  resumeSiteTitle: "Загрузки с {host} снова передаются в Downpipe",
  pausedBy: "Здесь приостановлено правилом {site}",
  progress: "Прогресс: {label}",
  downloadItem: "Загрузить {label} с помощью Downpipe",
  // The same words as the desktop app's states.
  state: {
    queued: "В очереди",
    starting: "Подключение",
    downloading: "Загрузка",
    waiting: "Повтор",
    verifying: "Проверка",
    paused: "Приостановлено",
    completed: "Готово",
    failed: "Ошибка",
    canceled: "Отменено",
  },
  outcome: {
    added: "Отправлено",
    duplicate: "В Downpipe",
    failed: "Не отправлено",
    kept: "В браузере",
  },
  waitingBadge: "Ожидает в Downpipe",
  needsAnswer: "Нужен ваш ответ",
  downloadFailed: "Загрузка не удалась",
  canceledInApp: "Отменено в Downpipe",
};
