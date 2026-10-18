// Problems the extension words itself. Messages the app sends over the
// browser bridge for codes not listed here are shown as the app wrote them.
import type { Messages } from "../core";

export const errors: Messages["errors"] = {
  bridge: {
    host_missing: "Downpipe tarayıcı köprüsü yüklü değil.",
    host_forbidden: "Tarayıcı köprüsü yüklü ama bu eklentiye izin vermiyor.",
    host_error: "Tarayıcı köprüsü beklenmedik şekilde durdu.",
    timeout: "Downpipe zamanında yanıt vermedi.",
    bad_response: "Tarayıcı köprüsü, bu eklentinin anlamadığı bir yanıt gönderdi.",
  },
  code: {
    app_not_running: "Downpipe çalışmıyor. Downpipe uygulamasını açıp yeniden deneyin.",
    app_unavailable: "Downpipe yanıt vermedi. Uygulamayı yeniden başlatıp tekrar deneyin.",
    duplicate: "Zaten Downpipe'ta.",
    invalid_url: "Bu bağlantı indirilemez. Yalnızca http ve https bağlantıları çalışır.",
    version_mismatch:
      "Downpipe ile tarayıcı köprüsünün sürümleri farklı. Kendi köprüsünü kaydetmesi için Downpipe'ı yeniden başlatın.",
    unsupported_version: "Bu eklenti, yüklü Downpipe'tan daha yeni. Downpipe'ı güncelleyin.",
    message_too_large: "Bağlantı, aktarılamayacak kadar uzun.",
    unsupported_page: "Downpipe bu siteden video indirmez.",
  },
  refused: "Downpipe indirmeyi reddetti ({code}).",
  streamsUnsupported: "Downpipe'ın bu sürümü akış indiremez. Downpipe'ı güncelleyin.",
  pagesUnsupported: "Downpipe'ın bu sürümü sayfalardaki videoları indiremez. Downpipe'ı güncelleyin.",
  keptClosed: "Downpipe çalışmadığı için dosyayı tarayıcı indirdi.",
  keptInstead: "{reason} Bunun yerine dosyayı tarayıcı indirdi.",
  notVideoPage: "Downpipe bu sayfadan video indirmez.",
  unknownVideoPage: "Bu, Downpipe'ın tanıdığı bir video sayfası değil.",
  mediaGone: "Bu medya artık listede yok. Yeniden oynatın.",
  mediaInvalid: "Bu medya indirilemez.",
  streamInvalid: "Bu akış indirilemez.",
};
