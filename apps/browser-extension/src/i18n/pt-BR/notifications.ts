// System notifications after a download was handed over, or not, and the toolbar button's tooltip.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Aguardando sua resposta no Downpipe",
  waitingBody: "Confirme o nome e a pasta no Downpipe para iniciar o download.",
  downloading: "Baixando no Downpipe",
  duplicate: "Já está no Downpipe",
  kept: "Baixado pelo navegador",
  failed: "Não enviado ao Downpipe",
  toolbarTitle: "Downpipe: {label}. {message}",
};
