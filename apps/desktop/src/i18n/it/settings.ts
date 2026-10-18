// The settings dialog, one group per tab, and the browser bridge's states.
// {placeholders} that hold an element (a path, a command) are put in by the dialog.
import type { Messages } from "../core";

export const settings: Messages["settings"] = {
  eyebrow: "Impostazioni",
  title: "Preferenze",
  tabsLabel: "Sezioni delle impostazioni",
  tabs: {
    downloads: "Download",
    queues: "Code",
    categories: "Categorie",
    network: "Rete",
    media: "Media",
    schedule: "Pianificazione",
    app: "App",
  },
  downloads: {
    folder: "Cartella predefinita",
    folderHint: "Le categorie qui sotto creano le loro cartelle all'interno di questa.",
    concurrent: "Download contemporanei",
    concurrentHint: "Gli altri attendono in coda. Da 1 a 16.",
    connections: "Connessioni per download",
    connectionsHint: "Limite massimo. Downpipe parte con 2 e ne aggiunge altre solo finché aiutano. Da 1 a 32.",
    speedLimit: "Limite di velocità",
    speedLimitHint: "Condiviso da tutti i download. Lascia vuoto per nessun limite. Si applica subito.",
    speedLimitInvalid: "Inserisci un numero, ad esempio 1,5 o 1.5.",
    noLimit: "Nessun limite",
    unit: "Unità del limite di velocità",
  },
  queues: {
    intro:
      "Ogni coda esegue il proprio numero di download contemporanei, in parallelo alle altre; all'interno di una coda, i download partono dall'alto verso il basso. Sposta i download tra le code e cambiane l'ordine dal menu del tasto destro. I download di una coda rimossa passano alla coda principale.",
    label: "Code",
    name: "Nome della coda",
    namePlaceholder: "Nome",
    mainLimit: { one: "{count} alla volta", many: "{count} alla volta", other: "{count} alla volta" },
    mainLimitTitle: "Si imposta in Download, Download contemporanei",
    limit: "Download contemporanei di {name}",
    limitUnnamed: "Download contemporanei della coda",
    limitTitle: "Download contemporanei, da 1 a 16",
    remove: "Rimuovi {name}",
    removeUnnamed: "Rimuovi coda",
    newName: "Coda {number}",
    add: "Aggiungi coda",
  },
  categories: {
    use: "Suddividi i download in cartelle per categoria",
    useHint:
      "Un nuovo download va nella cartella della sua categoria in base al tipo di file. Se il nome del file inviato dal server indica un tipo diverso, il file completato viene spostato nella cartella giusta. Le cartelle che scegli tu non vengono mai cambiate.",
    label: "Categorie",
    name: "Nome della categoria",
    namePlaceholder: "Nome",
    types: "Tipi di file di {name}",
    typesUnnamed: "Tipi di file della categoria",
    remove: "Rimuovi {name}",
    removeUnnamed: "Rimuovi categoria",
    folder: "Cartella di {name}",
    folderUnnamed: "Cartella della categoria",
    folderPlaceholder: "Cartella: {name} nella cartella predefinita",
    folderPlaceholderUnnamed: "Cartella: nome nella cartella predefinita",
    add: "Aggiungi categoria",
  },
  network: {
    proxy: "Proxy",
    proxyHint:
      "http://, https:// o socks5://host:port. Se vuoto, usa il proxy di sistema. Vale per i download avviati dopo il salvataggio.",
    proxyPlaceholder: "Proxy di sistema",
    userAgent: "User agent",
    userAgentHint: "Alcuni server rispondono solo ai browser. Se vuoto, invia quello di Downpipe.",
  },
  media: {
    ffmpegHint:
      "Unisce in un unico file il video e l'audio separati di un flusso, senza ricodifica. Se vuoto, usa ffmpeg dal PATH. Downpipe non include FFmpeg.",
    ffmpegPlaceholder: "ffmpeg nel PATH",
    found: "Trovato",
    notFound: "Non trovato",
    ffmpegMissing:
      "I flussi con video e audio separati vengono salvati come due file. Installa FFmpeg e aggiungilo al PATH, oppure inserisci sopra il percorso di ffmpeg, poi salva.",
    streams:
      "I flussi arrivano dal rilevamento dei contenuti multimediali dell'estensione del browser, oppure dai link .m3u8 e .mpd aggiunti qui. I flussi protetti da DRM e quelli in diretta non vengono scaricati.",
  },
  siteTools: {
    heading: "Video nelle pagine",
    intro:
      "In una pagina di YouTube, Scarica con Downpipe passa la pagina a yt-dlp, che ne estrae il video. yt-dlp ha bisogno di Deno per YouTube e di FFmpeg per unire video e audio oltre i 360p.",
    ytDlpHint: "Se vuoto, usa la copia scaricata qui sotto, altrimenti yt-dlp dal PATH.",
    ytDlpPlaceholder: "Copia scaricata, altrimenti yt-dlp nel PATH",
    denoHint:
      "Versione 2.3.0 o successiva. Se vuoto, usa la copia scaricata qui sotto, altrimenti deno dal PATH, altrimenti Node 22 o successivo.",
    denoPlaceholder: "Copia scaricata, altrimenti deno nel PATH",
    looking: "Ricerca di yt-dlp e Deno…",
    listLabel: "Strumenti per i video nelle pagine",
    downloading: "Download in corso…",
    // {folder} is either a path or toolsFolder below, so the sentence names the folder itself.
    installNote:
      "Solo quando lo premi: le build ufficiali da github.com/yt-dlp/yt-dlp e github.com/denoland/deno, ciascuna verificata con lo SHA-256 pubblicato dalla sua release, vengono salvate nella cartella {folder}. YouTube cambia spesso; premilo di nuovo per avere l'ultima versione di yt-dlp.",
    toolsFolder: "degli strumenti di Downpipe",
    cookies:
      "Downpipe non invia cookie a yt-dlp, quindi i video privati, riservati agli abbonati e con limiti di età non vengono scaricati, e nemmeno quelli protetti (DRM) e in diretta. Il rispetto dei termini di YouTube è una tua responsabilità.",
  },
  schedule: {
    enable: "Avvia e ferma i download secondo una pianificazione",
    enableHint:
      "All'ora di avvio, i download in pausa e non riusciti riprendono. All'ora di arresto, vengono messi in pausa. Downpipe deve essere in esecuzione, nella finestra o nell'area di notifica.",
    queue: "Coda",
    queueHint: "Le altre code non vengono toccate.",
    everyQueue: "Tutte le code",
    start: "Avvia alle",
    stop: "Ferma alle",
    stopHint: "Facoltativo",
    days: "Giorni",
  },
  app: {
    language: "Lingua",
    languageHint: "Si applica subito alla finestra, al menu dell'area di notifica e alle notifiche.",
    systemDefault: "Predefinita di sistema ({language})",
    notify: "Notifica quando un download termina",
    notifyHint: "Una notifica di sistema quando un download viene completato o non riesce mentre la finestra non è in primo piano.",
    tray: "Continua l'esecuzione nell'area di notifica quando la finestra viene chiusa",
    trayHint: "I download continuano e l'estensione del browser può ancora inviare link. Per uscire, usa l'icona nell'area di notifica.",
    startup: "Avvia Downpipe all'accesso",
    startupHint: "Si avvia nell'area di notifica, così i download intercettati hanno sempre dove andare.",
    clipboard: "Proponi di scaricare i link copiati",
    clipboardHint:
      "Quando copi un link a un tipo di file elencato in Categorie, o a un video di YouTube, Downpipe lo apre nella finestra Aggiungi. I link già presenti nell'elenco vengono ignorati.",
    awake: "Mantieni attivo il computer durante il download",
    awakeHint: "La sospensione attende che i download si fermino. Lo schermo può comunque spegnersi.",
    confirmBrowser: "Chiedi prima di avviare i download dal browser",
    confirmBrowserHint:
      "Scegli prima nome del file, cartella e coda, in una piccola finestra sopra il browser. Disattivato: partono subito con la cartella automatica.",
  },
  bridge: {
    heading: "Collegamento con il browser",
    register: "Registra nei browser all'avvio",
    registerHint:
      "Permette all'estensione Downpipe in Chrome, Edge, Chromium e Firefox di raggiungere questa copia dell'app. Modifica solo il tuo account utente. Disattivalo per gestire tu la registrazione con downpipe-native-host.",
    checking: "Verifica della registrazione…",
    noHost: "Questa copia di Downpipe non ha il collegamento con il browser in {host}. Reinstalla Downpipe; in una build di sviluppo, esegui {command}.",
    listLabel: "Registrazione per browser",
    registering: "Registrazione in corso…",
    registerNow: "Registra ora",
    pickUp: "I browser aperti la rilevano la prossima volta che l'estensione esegue il controllo.",
    states: {
      registered: "Registrato",
      notInstalled: "Non installato",
      missing: "Non registrato",
      invalid: "Illeggibile",
      otherHost: "Altra copia",
      extensionNotAllowed: "Altra estensione",
      failed: "Errore",
    },
  },
};
