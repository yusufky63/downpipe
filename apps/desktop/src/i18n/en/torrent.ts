// BitTorrent downloads: the list, the Add dialog, the file chooser, the
// details and the settings. "BitTorrent", "DHT", "PEX", "UPnP", "SOCKS5"
// and ".torrent" stay as written in every language.
export const torrent = {
  /** The kind of download, e.g. in "From the browser · Torrent". */
  kind: "Torrent",
  /** The state badge of a finished torrent that still uploads. */
  seeding: "Seeding",
  /** Details under a torrent's name in the list. */
  row: {
    fetching: "Asking peers for the file list",
    peers: { one: "{count} peer", other: "{count} peers" },
    /** {speed} is like "1.2 MiB/s". */
    upload: "Up {speed}",
    /** {ratio} is like "0.42": uploaded divided by downloaded. */
    ratio: "Ratio {ratio}",
  },
  stopSeeding: "Stop seeding",
  add: {
    openFile: "Open a .torrent file",
    tooLarge: "That file is too large to be a torrent.",
    /** A single magnet link in the Links field. */
    magnet: "A torrent. You choose its files next.",
    /** Added after the Links hint when only some links are magnet links. */
    some: {
      one: "One of them is a torrent: you choose its files next.",
      other: "{count} of them are torrents: you choose their files next.",
    },
    /** Every link is a magnet link. */
    all: "You choose the files of each torrent next.",
  },
  confirm: {
    title: "Download this torrent?",
    files: "Files",
    all: "All files",
    /** Under the file list: how many files are chosen and their size. */
    chosen: "{count} of {total} chosen · {size}",
    loadingMagnet: "Asking peers for the file list…",
    loading: "Reading the torrent…",
    /** {message} is the reason, ending with a full stop. */
    failed: "Could not get the file list: {message} Every file is downloaded once it starts.",
    noneChosen: "Choose at least one file.",
    /** {name} is the torrent's folder. */
    folder: "The files go into a folder named {name} inside it.",
    privacy: "Peers see your IP address while it downloads and seeds.",
    private: "A private torrent: only its trackers find peers.",
  },
  details: {
    infoHash: "Info hash",
    files: "Files",
    /** Chosen files of all. */
    filesValue: "{chosen} of {total}",
    peers: "Peers",
    peersValue: "{connected} connected, {known} known",
    uploaded: "Uploaded",
    /** {size} uploaded so far and the ratio, e.g. "120 MiB · ratio 0.42". */
    uploadedValue: "{size} · ratio {ratio}",
  },
  /** The progress window's status line. */
  progress: {
    fetching: "Asking peers for the file list",
    peers: { one: "Receiving data · {count} peer", other: "Receiving data · {count} peers" },
    seeding: "Finished, seeding to other peers",
  },
  settings: {
    tab: "BitTorrent",
    seed: "Seed after downloading",
    seedHint: "Keep uploading finished torrents to other peers until a limit below is reached. Seeding stops when Downpipe quits.",
    ratio: "Stop at ratio",
    ratioHint: "Uploaded divided by downloaded, e.g. 1.0. 0 for no ratio limit.",
    minutes: "Stop after minutes",
    minutesHint: "0 for no time limit.",
    uploadLimit: "Upload limit",
    uploadLimitHint: "For all torrents together. Empty for no limit. Downloads follow the speed limit under Downloads.",
    port: "Listening port",
    portHint: "For peers that connect to you. 0 picks a free port at every start.",
    dht: "DHT",
    dhtHint: "Find peers through the distributed hash table. Most magnet links need it.",
    lsd: "Local peer discovery",
    lsdHint: "Find peers on your local network.",
    upnp: "Forward the port on the router (UPnP)",
    upnpHint: "Lets more peers reach you. Off by default: it opens a port on your router.",
    pex: "Peer exchange (PEX) is always on for public torrents and never used for private ones; the BitTorrent library has no switch for it.",
    privacy: "Peers see your IP address. A SOCKS5 proxy under Network also carries peer connections; an HTTP proxy does not.",
    applies: "Port, DHT, local discovery and UPnP changes apply once no torrent is downloading or seeding.",
  },
};
