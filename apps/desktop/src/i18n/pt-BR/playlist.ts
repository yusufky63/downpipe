// The dialog that lists a playlist's or channel's videos to choose from.
import type { Messages } from "../core";

export const playlist: Messages["playlist"] = {
  eyebrow: {
    playlist: "Playlist do {site}",
    channel: "Canal do {site}",
  },
  untitled: "Playlist",
  reading: "Pedindo os vídeos ao yt-dlp…",
  unavailable: "Esta playlist ou este canal não está disponível: é privado, foi removido ou está bloqueado na sua região.",
  empty: "O yt-dlp não encontrou vídeos nesta lista.",
  truncated: { one: "Apenas o primeiro vídeo é listado.", many: "Apenas os primeiros {count} vídeos são listados.", other: "Apenas os primeiros {count} vídeos são listados." },
  filter: "Filtrar pelo título",
  selectAll: "Selecionar tudo",
  selectNone: "Desmarcar tudo",
  selected: { one: "{count} de {total} selecionado", many: "{count} de {total} selecionados", other: "{count} de {total} selecionados" },
  rowsLabel: "Vídeos",
  notAvailable: "Indisponível",
  untitledVideo: "Vídeo sem título",
  quality: "Qualidade",
  qualityHint: "Uma qualidade para todos os vídeos; o vídeo que não a tiver recebe a próxima mais baixa.",
  presets: {
    best: "Melhor",
    "1080p": "Até 1080p",
    "720p": "Até 720p",
    audio: "Somente áudio",
  },
  autoCaptions: "Usar legendas automáticas onde não houver legendas",
  noFfmpeg: "FFmpeg não encontrado: só é possível baixar qualidades com som próprio, o que no YouTube significa no máximo 360p.",
  folder: "Pasta",
  folderHint: "Criada dentro da pasta ao lado. Vazio: sem pasta própria.",
  saveIn: "Salvar em",
  submit: { one: "Baixar {count} vídeo", many: "Baixar {count} vídeos", other: "Baixar {count} vídeos" },
  addToList: { one: "Adicionar {count} vídeo", many: "Adicionar {count} vídeos", other: "Adicionar {count} vídeos" },
  added: { one: "{count} vídeo adicionado.", many: "{count} vídeos adicionados.", other: "{count} vídeos adicionados." },
  skipped: { one: "{count} já estava na lista.", many: "{count} já estavam na lista.", other: "{count} já estavam na lista." },
};
