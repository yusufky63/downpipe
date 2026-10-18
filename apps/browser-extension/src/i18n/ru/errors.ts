// Problems the extension words itself. "Browser bridge" is the native host,
// called "компонент связи с браузером" as in the desktop app.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "Компонент связи Downpipe с браузером не установлен.",
    host_forbidden: "Компонент связи с браузером установлен, но не разрешает работу с этим расширением.",
    host_error: "Компонент связи с браузером неожиданно завершил работу.",
    timeout: "Downpipe не ответил вовремя.",
    bad_response: "Компонент связи с браузером прислал ответ, который это расширение не понимает.",
  },
  code: {
    app_not_running: "Downpipe не запущен. Откройте приложение Downpipe и повторите попытку.",
    app_unavailable: "Downpipe не ответил. Перезапустите приложение и повторите попытку.",
    duplicate: "Уже есть в Downpipe.",
    invalid_url: "Эту ссылку нельзя загрузить. Поддерживаются только ссылки http и https.",
    version_mismatch:
      "Версии Downpipe и его компонента связи с браузером различаются. Перезапустите Downpipe, чтобы он зарегистрировал свой компонент.",
    unsupported_version: "Это расширение новее установленного Downpipe. Обновите Downpipe.",
    message_too_large: "Ссылка слишком длинная для передачи.",
    unsupported_page: "Downpipe не загружает видео с этого сайта.",
  },
  refused: "Downpipe отклонил загрузку ({code}).",
  streamsUnsupported: "Эта версия Downpipe не умеет загружать потоки. Обновите Downpipe.",
  pagesUnsupported: "Эта версия Downpipe не умеет загружать видео со страниц. Обновите Downpipe.",
  keptClosed: "Downpipe не был запущен, поэтому файл загрузил браузер.",
  keptInstead: "{reason} Вместо этого файл загрузил браузер.",
  notVideoPage: "Downpipe не загружает видео с этой страницы.",
  unknownVideoPage: "Downpipe не знает такой страницы с видео.",
  mediaGone: "Этого медиафайла больше нет в списке. Воспроизведите его снова.",
  mediaInvalid: "Этот медиафайл нельзя загрузить.",
  streamInvalid: "Этот поток нельзя загрузить.",
};
