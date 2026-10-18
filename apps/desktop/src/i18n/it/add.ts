// The dialog for adding links. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const add: Messages["add"] = {
  eyebrow: "Nuovo download",
  title: "Aggiungi un link",
  titleMany: { one: "Aggiungi {count} link", many: "Aggiungi {count} link", other: "Aggiungi {count} link" },
  addPaused: "Aggiungi in pausa",
  submit: {
    next: "Avanti",
    addToList: "Aggiungi all'elenco",
    download: "Scarica",
    downloadMany: { one: "Scarica {count} file", many: "Scarica {count} file", other: "Scarica {count} file" },
  },
  links: {
    playlist: "Una playlist di {site}. Nel passaggio successivo scegli i suoi video.",
    channel: "Un canale di {site}. Nel passaggio successivo scegli i suoi video.",
    label: "Link",
    hint: "Un link per riga, oppure uno schema come file[001-100].zip. Sono supportati solo http e https.",
    found: {
      one: "{count} link trovato. Un download per ciascuno.",
      many: "{count} link trovati. Un download per ciascuno.",
      other: "{count} link trovati. Un download per ciascuno.",
    },
    fromPattern: {
      one: "{count} link dallo schema. Un download per ciascuno.",
      many: "{count} link dallo schema. Un download per ciascuno.",
      other: "{count} link dallo schema. Un download per ciascuno.",
    },
    patternCapped: {
      one: "Viene usato solo il primo link dello schema.",
      many: "Vengono usati solo i primi {count} link dello schema.",
      other: "Vengono usati solo i primi {count} link dello schema.",
    },
    video: "Un video di {site}. Nel passaggio successivo ne scegli la qualità.",
    allVideos: "Nel passaggio successivo scegli la qualità di ogni video.",
    someVideos: {
      one: "Uno di questi è un video: nel passaggio successivo ne scegli la qualità.",
      many: "{count} di questi sono video: nel passaggio successivo ne scegli la qualità.",
      other: "{count} di questi sono video: nel passaggio successivo ne scegli la qualità.",
    },
    importList: "Importa un elenco",
    importHint: "Un file .txt con un link per riga. Puoi anche rilasciarlo qui.",
  },
  saveTo: {
    label: "Salva in",
    automatic: "Automatica: la cartella della categoria del file.",
  },
  fileName: {
    label: "Nome file",
    placeholder: "Automatico",
    hint: "Lascia vuoto per usare il nome inviato dal server.",
    videoHint: "Lascia vuoto per usare il titolo del video.",
    singleOnly: "Solo per un singolo link.",
  },
  connections: {
    label: "Connessioni",
    hint: "Limite massimo",
    videoHint: "Non per i video",
  },
  queue: {
    label: "Coda",
    hint: "Ogni coda esegue in parallelo i propri download.",
  },
  checksum: {
    label: "Checksum",
    hint: "Facoltativo, ad es. sha256:9f86d0… Il file viene rinominato nella posizione finale solo se corrisponde.",
    videoHint: "Solo per i file: yt-dlp compone il file di un video a partire dai formati del sito.",
  },
  errors: {
    listTooLarge: "Questo file è troppo grande per un elenco di link.",
    noLinks: "Incolla almeno un link http o https.",
    forLink: "{url}: {error}",
  },
};
