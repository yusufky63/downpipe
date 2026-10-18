// The connection to the desktop app, as the popup and the links page show it.
// The command lines after the "run:" texts are not translated.
import type { Messages } from "../core";

export const status: Messages["status"] = {
  checking: "Bağlantı denetleniyor",
  connected: "Downpipe'a bağlı",
  connectedDetail: "Uygulama {app}, tarayıcı köprüsü {bridge}",
  appNotRunning: "Downpipe çalışmıyor",
  appNotRunningDetail: "Downpipe uygulamasını açın. Uygulama kapalıyken gönderilen bağlantılar kuyruğa alınmaz.",
  hostMissing: "Tarayıcı köprüsü yüklü değil",
  hostMissingDetail: "Downpipe uygulamasını bir kez açın; köprüyü başlarken kaydeder. Kaynaktan derlenmiş bir sürüm için şunu çalıştırın:",
  hostForbidden: "Köprü bu eklentiye izin vermiyor",
  hostForbiddenDetail: "Köprüyü bu eklentinin kimliği için kaydedin, sonra tarayıcıyı yeniden başlatın:",
  error: "Köprü hatası",
};
