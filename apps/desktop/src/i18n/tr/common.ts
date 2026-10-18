// Words used across the app. {placeholders} are filled in at run time.
import type { Messages } from "../core";

export const common: Messages["common"] = {
  appName: "Downpipe",
  cancel: "İptal",
  save: "Kaydet",
  close: "Kapat",
  browse: "Gözat",
  dismiss: "Kapat",
  keep: "Koru",
  open: "Aç",
  pause: "Duraklat",
  resume: "Sürdür",
  retry: "Yeniden dene",
  remove: "Kaldır",
  showInFolder: "Klasörde göster",
  moreActions: "Diğer eylemler",
  unknown: "Bilinmiyor",
  none: "–",
  siteVideo: "{site} videosu",
  downloads: { one: "{count} indirme", other: "{count} indirme" },
  links: { one: "{count} bağlantı", other: "{count} bağlantı" },
};

export const states: Messages["states"] = {
  confirm: "Onay bekliyor",
  queued: "Sırada",
  starting: "Bağlanıyor",
  downloading: "İndiriliyor",
  waiting: "Yeniden deneniyor",
  verifying: "Doğrulanıyor",
  paused: "Duraklatıldı",
  completed: "Tamamlandı",
  failed: "Başarısız",
  canceled: "İptal edildi",
  why: {
    noData: "Sunucudan veri gelmiyor, yeniden bağlanılıyor",
    network: "Ağ bağlantısı yok, geri gelince yeniden denenecek",
    throttled: "Sunucu yavaşlamayı istedi, bekleniyor",
    server: "Sunucu hatası, yeniden deneniyor",
  },
  filters: {
    all: "Tümü",
    active: "Etkin",
    queued: "Sırada",
    completed: "Tamamlanan",
    failed: "Başarısız",
  },
};
