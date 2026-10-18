// Small dialogs: removing downloads, and fetching a finished file again.
import type { Messages } from "../core";

export const dialogs: Messages["dialogs"] = {
  remove: {
    deleteFile: "Kaldır ve dosyayı sil",
    deleteFiles: "Kaldır ve dosyaları sil",
    deletePartial: "Kaldır ve kısmi verileri sil",
    fromList: "Listeden kaldır",
    doneOne: "Listeden kaldırmak dosyayı diskte tutar. Silmek dosyanın kendisini de kaldırır.",
    doneMany: "Listeden kaldırmak dosyaları diskte tutar. Silmek dosyaların kendilerini de kaldırır.",
    partialOne: "Listeden kaldırmak kısmi dosyayı tutar; böylece aynı bağlantı daha sonra yine sürdürülebilir. Silmek kısmi verileri de kaldırır.",
    partialMany: "Listeden kaldırmak bitmiş dosyaları ve kısmi verileri diskte tutar. Silmek ikisini de kaldırır.",
  },
  redownload: {
    eyebrow: "Yeniden indir",
    body: "Dosyayı aynı bağlantıdan aynı klasöre yeniden indirir; örneğin sunucuda değiştiğinde ya da diskteki kopya bozulduğunda.",
    replace: "Mevcut dosyayı değiştir",
    replaceOn: "Önce mevcut dosya silinir ve yenisi onun adını alır.",
    replaceOff: "Kapalı: mevcut dosya kalır ve yeni kopya yanına kullanılmayan bir adla kaydedilir.",
    confirm: "Yeniden indir",
    confirmReplace: "Sil ve yeniden indir",
  },
  duplicate: {
    eyebrow: "Zaten listede",
    titleMany: { one: "{count} bağlantı zaten listede", other: "{count} bağlantı zaten listede" },
    body: "Bu adrese sahip bir indirme zaten listede ({state}). Ne yapmak istersiniz?",
    bodyMany: {
      one: "Aynı adrese sahip bir indirme zaten listede. Ne yapmak istersiniz?",
      other: "Bu adreslere sahip indirmeler zaten listede. {count} indirmenin tümüyle ne yapmak istersiniz?",
    },
    question: "Bağlantıyla ne yapılsın",
    copy: "Numaralı bir dosya adıyla yeniden ekle",
    overwrite: "Yeniden ekle ve mevcut dosyayı değiştir",
    existing: "Mevcut indirme bitmişse göster, bitmemişse sürdür",
    ok: "Tamam",
  },
};
