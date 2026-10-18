// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Verifica della connessione",
  connected: "Connesso a Downpipe",
  connectedDetail: "App {app}, bridge del browser {bridge}",
  appNotRunning: "Downpipe non è in esecuzione",
  appNotRunningDetail: "Apri l'app Downpipe. I link inviati mentre è chiusa non vengono messi in coda.",
  hostMissing: "Bridge del browser non installato",
  hostMissingDetail:
    "Apri l'app Downpipe una volta: registra il bridge all'avvio. Per una build compilata dai sorgenti, esegui:",
  hostForbidden: "Il bridge non consente questa estensione",
  hostForbiddenDetail: "Registralo per l'id di questa estensione, poi riavvia il browser:",
  error: "Errore del bridge",
};
