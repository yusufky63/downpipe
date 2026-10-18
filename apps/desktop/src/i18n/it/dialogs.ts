// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "Rimuovi ed elimina il file",
    deleteFiles: "Rimuovi ed elimina i file",
    deletePartial: "Rimuovi ed elimina i dati parziali",
    fromList: "Rimuovi dall'elenco",
    doneOne: "Se lo rimuovi dall'elenco, il file resta sul disco. Se lo elimini, viene rimosso anche il file stesso.",
    doneMany: "Se li rimuovi dall'elenco, i file restano sul disco. Se li elimini, vengono rimossi anche i file stessi.",
    partialOne:
      "Se lo rimuovi dall'elenco, il file parziale resta, così lo stesso link potrà ancora riprendere in seguito. Se lo elimini, vengono rimossi anche i dati parziali.",
    partialMany: "Se li rimuovi dall'elenco, i file completati e i dati parziali restano sul disco. Se li elimini, vengono rimossi entrambi.",
  },
  redownload: {
    eyebrow: "Scarica di nuovo",
    body: "Scarica di nuovo il file dallo stesso link nella stessa cartella, ad esempio quando è cambiato sul server o la copia sul disco è danneggiata.",
    replace: "Sostituisci il file esistente",
    replaceOn: "Il file attuale viene eliminato prima, e quello nuovo ne prende il nome.",
    replaceOff: "Disattivato: il file attuale resta, e la nuova copia viene salvata accanto con un nome libero.",
    confirm: "Scarica di nuovo",
    confirmReplace: "Elimina e scarica di nuovo",
  },
  duplicate: {
    eyebrow: "Già nell'elenco",
    titleMany: {
      one: "{count} link è già nell'elenco",
      many: "{count} link sono già nell'elenco",
      other: "{count} link sono già nell'elenco",
    },
    body: "Un download con questo indirizzo è già nell'elenco ({state}). Cosa vuoi fare?",
    bodyMany: {
      one: "Un download con lo stesso indirizzo è già nell'elenco. Cosa vuoi fare?",
      many: "Ci sono già nell'elenco download con questi indirizzi. Cosa vuoi fare con tutti e {count}?",
      other: "Ci sono già nell'elenco download con questi indirizzi. Cosa vuoi fare con tutti e {count}?",
    },
    question: "Cosa fare con il link",
    copy: "Aggiungilo di nuovo con un nome file numerato",
    overwrite: "Aggiungilo di nuovo e sostituisci il file esistente",
    existing: "Mostra il download esistente se è completato, altrimenti riprendilo",
    ok: "OK",
    remember: "Non chiedere più, fai sempre così",
    rememberHint: "Si cambia di nuovo in Impostazioni, Download.",
  },
  /** A download's own speed limit, under the global one. {limit} is like "2 MiB/s". */
  speedLimit: {
    eyebrow: "Limite di velocità",
    body: "Solo per questo download, sotto il limite globale di {limit}.",
    bodyNoGlobal: "Solo per questo download. Non c'è un limite globale.",
    bodyMany: "Per ciascuno separatamente, sotto il limite globale di {limit}.",
    bodyManyNoGlobal: "Per ciascuno separatamente. Non c'è un limite globale.",
    label: "Limite",
    unit: "Unità del limite di velocità",
    placeholder: "Nessun limite",
    video: "Un video applica il nuovo limite quando riparte.",
    remove: "Togli limite",
    save: "Imposta limite",
  },
  /** "Pause until…" with a time of the user's choice. */
  pauseUntil: {
    eyebrow: "Metti in pausa fino a",
    label: "Riprendi alle",
    hint: "Va in pausa ora e riprende da solo a quest'ora mentre Downpipe è aperto, altrimenti appena si riapre.",
    past: "Scegli un orario futuro.",
    invalid: "Inserisci una data e un orario.",
    submit: "Metti in pausa",
  },
};
