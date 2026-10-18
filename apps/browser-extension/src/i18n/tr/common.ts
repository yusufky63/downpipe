// Words used on several screens. {placeholders} are filled in at run time;
// product names (Downpipe), formats (HLS, MP4) and units (KiB) stay as they are.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  download: "İndir",
  sending: "Gönderiliyor…",
  checkAgain: "Yeniden denetle",
  setupGuide: "Kurulum kılavuzu",
  filters: "Filtreler",
  unit: "Birim",
  extensionNoAnswer: "Eklenti yanıt vermedi.",
  justNow: "az önce",
  minutesAgo: { one: "{count} dk önce", other: "{count} dk önce" },
  hoursAgo: { one: "{count} sa önce", other: "{count} sa önce" },
};

export const results: Messages["results"] = {
  added: "Downpipe'a eklendi",
  waiting: "Downpipe'ta yanıtınız bekleniyor",
  sent: "Downpipe'a gönderildi",
};
