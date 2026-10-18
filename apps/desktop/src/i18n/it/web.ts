// Trovare file sul web: il grabber di siti, le iscrizioni ai feed e gli host di file.
import type { Messages } from "../core";

export const grabber: Messages["grabber"] = {
  open: "Grabber di siti",
  eyebrow: "Grabber di siti",
  title: "Trova file in un sito",
  address: {
    label: "Indirizzo di partenza",
    hint: "Downpipe legge questa pagina e ne segue i link. Non invia cookie, quindi vede ciò che vede un visitatore senza accesso.",
  },
  depth: {
    label: "Profondità",
    hint: "1 legge solo questa pagina; ogni livello segue un ulteriore livello di link.",
  },
  pages: {
    label: "Limite di pagine",
    hint: "Pagine lette al massimo",
  },
  types: {
    label: "Tipi di file",
    custom: "Altri tipi",
    customPlaceholder: "es. iso, torrent",
  },
  sameSite: {
    label: "Resta su questo sito",
    hint: "Si leggono solo le pagine del sito dell'indirizzo di partenza. I file collegati da altri siti vengono comunque elencati.",
  },
  robots: {
    label: "Rispetta robots.txt",
    hint: "Salta le pagine che un sito chiede ai crawler di evitare.",
  },
  sizes: {
    label: "Rileva le dimensioni",
    hint: "Una piccola richiesta per file, per i primi {count} file.",
  },
  delay: {
    label: "Attesa",
    hint: "Millisecondi tra due richieste allo stesso sito. I siti possono bloccare i visitatori troppo frettolosi.",
  },
  start: "Trova file",
  stop: "Ferma",
  again: "Nuova ricerca",
  progress: {
    pages: { one: "{done} di {count} pagina lette", many: "{done} di {count} pagine lette", other: "{done} di {count} pagine lette" },
    sizes: {
      one: "Rilevamento dimensioni: {done} di {count} file",
      many: "Rilevamento dimensioni: {done} di {count} file",
      other: "Rilevamento dimensioni: {done} di {count} file",
    },
    files: { one: "{count} file trovato", many: "{count} file trovati", other: "{count} file trovati" },
  },
  results: {
    title: { one: "{count} file trovato", many: "{count} file trovati", other: "{count} file trovati" },
    none: "Non sono stati trovati file di questi tipi.",
    canceled: "Fermato in anticipo: ecco i file trovati fino a quel momento.",
    pageLimit: "Il limite di pagine è stato raggiunto prima di leggerle tutte.",
    blocked: {
      one: "{count} pagina o file saltato perché robots.txt non lo consente.",
      many: "{count} pagine o file saltati perché robots.txt non lo consente.",
      other: "{count} pagine o file saltati perché robots.txt non lo consente.",
    },
    failed: { one: "Non è stato possibile leggere {count} pagina.", many: "Non è stato possibile leggere {count} pagine.", other: "Non è stato possibile leggere {count} pagine." },
    filter: "Filtra file",
    selectAll: "Seleziona tutti i file mostrati",
    name: "Nome",
    size: "Dimensione",
    foundOn: "Trovato in",
    selected: { one: "{count} selezionato, {size}", many: "{count} selezionati, {size}", other: "{count} selezionati, {size}" },
    unknownSizes: { one: "{count} di dimensione sconosciuta", many: "{count} di dimensione sconosciuta", other: "{count} di dimensione sconosciuta" },
    add: { one: "Aggiungi {count} file", many: "Aggiungi {count} file", other: "Aggiungi {count} file" },
    added: { one: "{count} file aggiunto.", many: "{count} file aggiunti.", other: "{count} file aggiunti." },
    alreadyListed: { one: "{count} era già nell'elenco.", many: "{count} erano già nell'elenco.", other: "{count} erano già nell'elenco." },
  },
  errors: {
    invalid_url: "Inserisci un indirizzo http o https.",
    no_types: "Scegli almeno un tipo di file.",
    robots_blocked: "Il robots.txt di questo sito chiede ai crawler di non leggere questa pagina. Disattiva Rispetta robots.txt per leggerla comunque.",
    start_page: "Impossibile leggere la pagina di partenza: {error}",
    not_html: "L'indirizzo di partenza non è una pagina web. Aggiungilo invece come download.",
  },
};

export const feeds: Messages["feeds"] = {
  open: "Feed",
  eyebrow: "Feed",
  title: "Feed RSS e Atom",
  intro: "Downpipe controlla ogni feed al suo intervallo e aggiunge i file delle nuove voci alla coda che scegli. Ogni voce viene aggiunta una sola volta, anche se rimuovi il suo download.",
  empty: "Ancora nessun feed. Aggiungi un podcast, un feed di rilasci o qualsiasi altro indirizzo RSS o Atom.",
  add: "Aggiungi un feed",
  checkNow: "Controlla ora",
  edit: "Modifica",
  off: "Disattivato",
  checked: "Controllato {date}",
  notChecked: "Non ancora controllato",
  error: "L'ultimo controllo non è riuscito: {error}",
  addedTotal: {
    one: "{count} download aggiunto in tutto",
    many: "{count} download aggiunti in tutto",
    other: "{count} download aggiunti in tutto",
  },
  checkResult: {
    none: "Nessuna nuova voce in {name}.",
    added: {
      one: "{count} download aggiunto da {name}.",
      many: "{count} download aggiunti da {name}.",
      other: "{count} download aggiunti da {name}.",
    },
  },
  confirmRemove: "Rimuovere {name}? I download che ha aggiunto restano nell'elenco.",
  form: {
    newTitle: "Nuovo feed",
    editTitle: "Modifica feed",
    url: "Indirizzo del feed",
    types: "Tipi di file",
    typesHint: "Vuoto: qualsiasi file offerto dalle voci. Per esempio mp3, m4a.",
    pattern: "Filtro titolo",
    patternHint: "Un'espressione regolare che il titolo deve contenere, senza distinguere maiuscole e minuscole, per esempio episodio|bonus. Vuoto: qualsiasi titolo.",
    interval: "Controlla ogni",
    queue: "Coda",
    enabled: "Controlla questo feed",
    enabledHint: "Disattivato: il feed resta nell'elenco ma non viene controllato.",
    existing: "Scarica ciò che contiene ora",
    existingHint: "Disattivato: vengono scaricate solo le voci pubblicate da ora in poi.",
    save: "Salva",
    subscribe: "Aggiungi feed",
    subscribed: {
      one: "Feed aggiunto. {count} download aggiunto.",
      many: "Feed aggiunto. {count} download aggiunti.",
      other: "Feed aggiunto. {count} download aggiunti.",
    },
    subscribedNone: "Feed aggiunto. Da ora in poi verranno scaricate le nuove voci.",
  },
  interval: {
    minutes: { one: "{count} minuto", many: "{count} minuti", other: "{count} minuti" },
    hours: { one: "{count} ora", many: "{count} ore", other: "{count} ore" },
    days: { one: "{count} giorno", many: "{count} giorni", other: "{count} giorni" },
  },
  errors: {
    duplicate_feed: "Questo feed è già nell'elenco.",
    not_a_feed: "Questo indirizzo è una pagina web, non un feed RSS o Atom.",
    invalid_url: "Inserisci un indirizzo http o https.",
  },
};
