// Words used on several screens, in Brazilian Portuguese. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "Baixar",
  sending: "Enviando…",
  checkAgain: "Verificar novamente",
  setupGuide: "Guia de configuração",
  filters: "Filtros",
  unit: "Unidade",
  extensionNoAnswer: "A extensão não respondeu.",
  justNow: "agora mesmo",
  minutesAgo: { one: "há {count} min", many: "há {count} min", other: "há {count} min" },
  hoursAgo: { one: "há {count} h", many: "há {count} h", other: "há {count} h" },
};

export const results: Messages["results"] = {
  added: "Adicionado ao Downpipe",
  waiting: "Aguardando sua resposta no Downpipe",
  sent: "Enviado ao Downpipe",
};
