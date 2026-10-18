// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "Плейлист {site}",
    channel: "Канал {site}",
  },
  untitled: "Плейлист",
  reading: "Запрос списка видео у yt-dlp…",
  unavailable: "Этот плейлист или канал недоступен: он личный, удален или заблокирован в вашем регионе.",
  empty: "yt-dlp не нашел видео в этом списке.",
  truncated: { one: "Показаны только первые видео: {count}.", few: "Показаны только первые видео: {count}.", many: "Показаны только первые видео: {count}.", other: "Показаны только первые видео: {count}." },
  filter: "Фильтр по названию",
  selectAll: "Выбрать все",
  selectNone: "Снять выбор",
  selected: { one: "Выбрано {count} из {total}", few: "Выбрано {count} из {total}", many: "Выбрано {count} из {total}", other: "Выбрано {count} из {total}" },
  rowsLabel: "Видео",
  notAvailable: "Недоступно",
  untitledVideo: "Видео без названия",
  quality: "Качество",
  qualityHint: "Одно качество для всех видео; видео без него получит ближайшее меньшее.",
  presets: {
    best: "Лучшее",
    "1080p": "До 1080p",
    "720p": "До 720p",
    audio: "Только звук",
  },
  autoCaptions: "Использовать автоматические субтитры, если других нет",
  noFfmpeg: "FFmpeg не найден: можно загрузить только варианты качества со своим звуком, а на YouTube это не больше 360p.",
  folder: "Папка",
  folderHint: "Создается внутри папки рядом. Пусто: без отдельной папки.",
  saveIn: "Сохранить в",
  submit: { one: "Загрузить {count} видео", few: "Загрузить {count} видео", many: "Загрузить {count} видео", other: "Загрузить {count} видео" },
  addToList: { one: "Добавить {count} видео", few: "Добавить {count} видео", many: "Добавить {count} видео", other: "Добавить {count} видео" },
  added: { one: "Добавлено {count} видео.", few: "Добавлено {count} видео.", many: "Добавлено {count} видео.", other: "Добавлено {count} видео." },
  skipped: { one: "Уже в списке: {count}.", few: "Уже в списке: {count}.", many: "Уже в списке: {count}.", other: "Уже в списке: {count}." },
};
