// Problems the extension words itself. Messages the app sends over the
// browser bridge for codes not listed here are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "Il bridge del browser di Downpipe non è installato.",
    host_forbidden: "Il bridge del browser è installato ma non consente questa estensione.",
    host_error: "Il bridge del browser si è arrestato in modo imprevisto.",
    timeout: "Downpipe non ha risposto in tempo.",
    bad_response: "Il bridge del browser ha inviato una risposta che questa estensione non comprende.",
  },
  code: {
    app_not_running: "Downpipe non è in esecuzione. Apri l'app Downpipe e riprova.",
    app_unavailable: "Downpipe non ha risposto. Riavvia l'app e riprova.",
    duplicate: "Già in Downpipe.",
    invalid_url: "Questo link non può essere scaricato. Funzionano solo i link http e https.",
    version_mismatch:
      "Downpipe e il suo bridge del browser hanno versioni diverse. Riavvia Downpipe in modo che registri il proprio bridge.",
    unsupported_version: "Questa estensione è più recente della versione di Downpipe installata. Aggiorna Downpipe.",
    message_too_large: "Il link è troppo lungo per essere inviato.",
    unsupported_page: "Downpipe non scarica video da questo sito.",
  },
  refused: "Downpipe ha rifiutato il download ({code}).",
  streamsUnsupported: "Questa versione di Downpipe non può scaricare flussi. Aggiorna Downpipe.",
  pagesUnsupported: "Questa versione di Downpipe non può scaricare video dalle pagine. Aggiorna Downpipe.",
  keptClosed: "Downpipe non era in esecuzione, quindi l'ha scaricato il browser.",
  keptInstead: "{reason} L'ha scaricato il browser.",
  notVideoPage: "Downpipe non scarica video da questa pagina.",
  unknownVideoPage: "Questa non è una pagina video che Downpipe conosce.",
  mediaGone: "Questo contenuto non è più nell'elenco. Riproducilo di nuovo.",
  mediaInvalid: "Questo contenuto non può essere scaricato.",
  streamInvalid: "Questo flusso non può essere scaricato.",
};
