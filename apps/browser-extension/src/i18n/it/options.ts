// The options page (Filters). In the help texts, {example}, {tarGz} and
// {wildcard} are code samples the page shows as they are.
import type { Messages } from "../core";

export const options: Messages["options"] = {
  pageTitle: "Filtri · Downpipe",
  headLabel: "Estensione del browser",
  lede:
    "Scegli quali download il browser passa a Downpipe mentre Cattura i download del browser è attivo, e quali media l'estensione elenca mentre Media in questa pagina è attivo. Tutto il resto rimane nel browser come al solito.",
  captureOff:
    "La cattura è disattivata. Questi filtri si applicano quando attivi Cattura i download del browser nel popup di Downpipe sulla barra degli strumenti.",
  loadError: "Impossibile caricare le impostazioni. Ricarica questa pagina per riprovare.",
  typesHeading: "01 — Tipi di file",
  captureAll: "Cattura tutti i tipi di file",
  captureAllDetail: "Ogni download avviato dal browser va a Downpipe, qualunque sia il tipo.",
  typesLabel: "Solo questi tipi di file",
  typesHelp:
    "Estensioni separate da spazi o virgole, ad esempio {example}. Un download viene catturato quando il nome del file termina con una di esse; funziona anche {tarGz}.",
  restoreDefault: "Ripristina l'elenco predefinito",
  sitesHeading: "02 — Siti esclusi",
  sitesLabel: "Non catturare né elencare mai i media su questi siti",
  sitesHelp:
    "Un sito per riga. {wildcard} copre anche tutti i sottodomini. Un download resta nel browser quando proviene da uno di questi siti o viene avviato in una delle loro pagine, e i media che riproducono non vengono elencati. Porte e percorsi vengono ignorati.",
  sizeHeading: "03 — Dimensione minima",
  sizeLabel: "Lascia al browser i file più piccoli",
  sizeHelp: "0 disattiva questa opzione. Quando il server non indica la dimensione di un file, questo va a Downpipe.",
  mediaHeading: "04 — Media nelle pagine",
  mediaHelp:
    "Usato mentre Media in questa pagina è attivo nel popup della barra degli strumenti. I flussi HLS e DASH vengono sempre elencati; anche qui valgono i siti esclusi indicati sopra.",
  overlay: "Mostra un pulsante Scarica sui video",
  overlayDetail: "Su un video in riproduzione, per qualche secondo e mentre il puntatore è sopra.",
  overlayHover: "Mostralo solo quando il puntatore è sul video",
  overlayHoverDetail: "Altrimenti appare anche per qualche secondo quando un video inizia.",
  overlaySitesLabel: "Siti senza il pulsante",
  overlaySitesHelp: "Un sito per riga, come per i siti esclusi. Nascondi su questo sito, nel menu del pulsante, aggiunge il sito qui. I contenuti riprodotti da questi siti restano elencati nel popup.",
  mediaTypesLabel: "Elenca i file multimediali di questi tipi",
  mediaTypesHelp:
    "Estensioni separate da spazi o virgole. Un file senza estensione viene valutato in base al tipo inviato dal server. Con un elenco vuoto vengono elencati solo i flussi.",
  mediaSizeLabel: "Escludi i file multimediali più piccoli di",
  mediaSizeHelp:
    "Tiene fuori i suoni di notifica e le brevi anteprime. 0 elenca tutte le dimensioni; i file di cui il server non indica la dimensione vengono sempre elencati.",
  bypassHeading: "05 — Scarica nel browser",
  bypassLabel: "Lascia che il browser scarichi un link cliccato con",
  bypassAlt: "Alt (Opzione su Mac)",
  bypassShift: "Maiusc",
  bypassOff: "Disattivato",
  bypassHelp: "Con la cattura attiva, un clic su un link tenendo premuto questo tasto lascia quel download al browser. Gli altri clic e le scorciatoie del browser restano come sono.",
  bypassAccess: "Serve l'accesso ai siti che visiti per accorgersi del clic. Non viene letto né inviato nulla finché non clicchi un link in questo modo.",
  bypassAllow: "Consenti l'accesso",
  save: "Salva",
  moreErrors: { one: "E {count} altro.", many: "E altri {count}.", other: "E altri {count}." },
  unsaved: "Modifiche non salvate",
  notSavedFix: "Non salvato. Correggi i campi evidenziati in rosso.",
  saving: "Salvataggio…",
  notSavedNoAnswer: "Non salvato. L'estensione non ha risposto; riprova.",
  saved: "Salvato",
  errors: {
    notExtension: "\"{value}\" non è un'estensione di file.",
    tooManyTypes: {
      one: "Elenca al massimo {count} tipo di file.",
      many: "Elenca al massimo {count} tipi di file.",
      other: "Elenca al massimo {count} tipi di file.",
    },
    notSite: "Riga {line}: \"{value}\" non è un sito. Usa un host come example.com o *.example.com.",
    tooManySites: {
      one: "Elenca al massimo {count} sito.",
      many: "Elenca al massimo {count} siti.",
      other: "Elenca al massimo {count} siti.",
    },
    sizeNotNumber: "Inserisci un numero come 2 o 1,5, oppure 0 per disattivare questa opzione.",
    sizeTooBig: "Inserisci una dimensione inferiore a 1 TiB.",
    noTypes: "Aggiungi almeno un tipo di file, oppure attiva Cattura tutti i tipi di file.",
  },
};
