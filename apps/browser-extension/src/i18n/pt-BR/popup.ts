// The toolbar popup. Its switch labels are quoted on the options page and in the media list.
import type { Messages } from "../core";

export const popup: Messages["popup"] = {
  connected: "Conectado",
  connectedTitle: "{title}. {detail}",
  desktopApp: "Aplicativo da área de trabalho",
  mediaHeading: "Mídia nesta página",
  detectMedia: "Detectar mídia nas páginas",
  downloadsHeading: "Downloads",
  pageLinks: "Links nesta página…",
  emptyActivity: "Clique com o botão direito em um link e escolha Baixar com o Downpipe.",
  capture: "Capturar downloads do navegador",
  captureDetail: "O navegador fica com eles quando o Downpipe está fechado.",
  notifications: "Notificações",
  notificationsDetail: "Quando um download é enviado, ou não.",
  // Short so the row stays on one line; the host is in the tooltip.
  pauseSite: "Pausar captura neste site",
  resumeSite: "Retomar captura neste site",
  pauseSiteTitle: "Downloads de {host}, ou iniciados nas páginas dele, ficam no navegador",
  resumeSiteTitle: "Downloads de {host} voltam a ir para o Downpipe",
  pausedBy: "Pausado aqui por {site}",
  progress: "Progresso de {label}",
  downloadItem: "Baixar {label} com o Downpipe",
  state: {
    queued: "Na fila",
    starting: "Iniciando",
    downloading: "Baixando",
    waiting: "Aguardando",
    verifying: "Verificando",
    paused: "Pausado",
    completed: "Concluído",
    failed: "Falhou",
    canceled: "Cancelado",
  },
  outcome: {
    added: "Enviado",
    duplicate: "No Downpipe",
    failed: "Não enviado",
    kept: "No navegador",
  },
  // Short: the badge leaves it little room.
  waitingBadge: "Aguardando no Downpipe",
  needsAnswer: "Precisa da sua resposta",
  downloadFailed: "O download falhou",
  canceledInApp: "Cancelado no Downpipe",
};
