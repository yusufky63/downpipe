// System notifications after a download was handed over, or not, and the
// toolbar button's tooltip. The download's name comes first in each message.
import type { Messages } from "../core";

export const notifications: Messages["notifications"] = {
  waitingTitle: "Downpipe'ta yanıtınız bekleniyor",
  waitingBody: "İndirmeyi başlatmak için adını ve klasörünü Downpipe'ta onaylayın.",
  downloading: "Downpipe'ta indiriliyor",
  duplicate: "Zaten Downpipe'ta",
  kept: "Tarayıcı tarafından indirildi",
  failed: "Downpipe'a gönderilmedi",
  toolbarTitle: "Downpipe: {label}. {message}",
};
