// The details dialog: progress, live numbers, the new-link form and the file's facts.
import type { Messages } from "../core";

export const details: Messages["details"] = {
  eyebrow: "Сведения о загрузке",
  openFile: "Открыть файл",
  remove: "Удалить…",
  cancelAndDelete: "Отменить и удалить",
  cancelAndDeleteTitle: "Останавливает загрузку и удаляет частично загруженный файл",
  downloadAgain: "Загрузить заново",
  redownload: "Загрузить заново…",
  progress: "Прогресс",
  progressOf: "{downloaded} из {total}",
  progressOfAbout: "{downloaded} из примерно {total}",
  legend: {
    downloaded: "Загружено",
    connection: "Подключение",
    remaining: "Осталось",
  },
  stats: {
    speed: "Скорость",
    timeLeft: "Осталось времени",
    connections: "Подключения",
    target: "цель: {count}",
    resume: "Докачка",
    supported: "Поддерживается",
    notSupported: "Не поддерживается",
    size: "Размер",
    downloaded: "Загружено",
  },
  rows: {
    savedTo: "Сохранено в",
    folder: "Папка",
    source: "Источник",
    video: "Видео",
    stream: "Поток",
    bestQuality: "Лучшее качество",
    // "из 21 фрагмента", "из 22 фрагментов": the genitive after "из".
    fragments: {
      one: "{done} из {count} фрагмента",
      few: "{done} из {count} фрагментов",
      many: "{done} из {count} фрагментов",
      other: "{done} из {count} фрагмента",
    },
    queue: "Очередь",
    siteRule: "Правило сайта",
    checksum: "Контрольная сумма",
    added: "Добавлено",
    finished: "Завершено",
  },
  relink: {
    button: "Указать новую ссылку…",
    label: "Новая ссылка для этого файла",
    submit: "Продолжить",
    hint: "Для устаревшей или нерабочей ссылки. Загрузка продолжится с того же места, только если новая ссылка отдает тот же файл; иначе ничего не будет перезаписано.",
  },
  copy: "Копировать",
  copied: "Скопировано",
};
