// Words used across the app, in Brazilian Portuguese. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "Cancelar",
  save: "Salvar",
  close: "Fechar",
  browse: "Procurar",
  dismiss: "Dispensar",
  keep: "Manter",
  open: "Abrir",
  pause: "Pausar",
  resume: "Retomar",
  retry: "Tentar novamente",
  remove: "Remover",
  showInFolder: "Mostrar na pasta",
  moreActions: "Mais ações",
  unknown: "Desconhecido",
  none: "–",
  siteVideo: "Vídeo do {site}",
  downloads: { one: "{count} download", many: "{count} downloads", other: "{count} downloads" },
  links: { one: "{count} link", many: "{count} links", other: "{count} links" },
};

export const states: Messages["states"] = {
  confirm: "Confirmar",
  queued: "Na fila",
  starting: "Conectando",
  downloading: "Baixando",
  waiting: "Tentando novamente",
  verifying: "Verificando",
  paused: "Pausado",
  completed: "Concluído",
  failed: "Falhou",
  canceled: "Cancelado",
  why: {
    noData: "O servidor não está enviando dados, reconectando",
    network: "Sem rede, nova tentativa quando ela voltar",
    throttled: "O servidor pediu para ir mais devagar, aguardando",
    server: "Erro do servidor, tentando de novo",
  },
  filters: {
    all: "Todos",
    active: "Ativos",
    queued: "Na fila",
    completed: "Concluídos",
    failed: "Com falha",
  },
};
