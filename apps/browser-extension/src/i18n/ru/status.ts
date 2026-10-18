// The connection to the desktop app. The command lines after the
// "выполните:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Проверка подключения",
  connected: "Подключено к Downpipe",
  connectedDetail: "Приложение {app}, компонент связи {bridge}",
  appNotRunning: "Downpipe не запущен",
  /** Instead of appNotRunningDetail while the bridge may start Downpipe. */
  appNotRunningStarts: "Отправленная загрузка запустит его в области уведомлений.",
  appNotRunningDetail: "Откройте приложение Downpipe. Ссылки, отправленные при закрытом приложении, в очередь не ставятся.",
  hostMissing: "Компонент связи с браузером не установлен",
  hostMissingDetail:
    "Один раз откройте приложение Downpipe: при запуске оно регистрирует компонент связи. Для сборки из исходного кода выполните:",
  hostForbidden: "Компонент связи не разрешает работу с этим расширением",
  hostForbiddenDetail: "Зарегистрируйте его для идентификатора этого расширения, затем перезапустите браузер:",
  error: "Ошибка компонента связи",
};
