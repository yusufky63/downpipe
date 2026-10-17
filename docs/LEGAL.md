# Legal notes

## Intended use

Downpipe is intended for downloading files and media that you own, have permission to download, that are made downloadable by the relevant service, or that may otherwise be lawfully downloaded. The project does not provide DRM circumvention and does not guarantee that use with a supported website is permitted by that website's terms.

Downpipe; sahibi olduğunuz, indirme izniniz bulunan, ilgili hizmetin indirilebilir olarak sunduğu ya da başka bir şekilde yasal olarak indirilebilen dosya ve medyaları indirmek için tasarlanmıştır. Proje DRM korumasını aşma imkânı sunmaz ve desteklenen bir sitede kullanımın o sitenin kullanım koşullarınca serbest olduğunu garanti etmez.

## What the project will not build

- DRM, encryption or paywall circumvention. Protected media is reported as unsupported: Downpipe refuses HLS streams with `SAMPLE-AES` or any key system other than plain AES-128 (whose key any player receives as an ordinary file) and DASH manifests with `ContentProtection`, before any media is requested, and the extension's Download button does not appear on videos that use Encrypted Media Extensions. The YouTube provider refuses videos yt-dlp reports as DRM-protected (purchased and rented films, for example) and leaves protected formats out of the quality list.
- Ways around anti-bot or other security measures.
- Silent collection of browser cookies or credentials.
- A central server that downloads on users' behalf. Downpipe is local-first: links, cookies and files stay on the user's machine.

## Platform terms

Media detection finds the unprotected streams a page plays on any site, X and news sites included. That this works technically does not mean the site allows downloading, and the same holds for site extractors. Several platforms restrict downloading or automated access in their terms, including YouTube, X and Meta.

**YouTube.** Downpipe can hand a YouTube video page to yt-dlp. [YouTube's Terms of Service](https://www.youtube.com/t/terms) do not allow downloading content except as the service expressly allows (its own download feature, for example) or with prior written permission from YouTube and, where applicable, the rights holders. Whether a download is allowed is between you, YouTube and the people who made the video; following YouTube's terms is your responsibility. Downpipe does not claim that videos from YouTube may be downloaded freely. It sends no cookies, so it downloads only what YouTube serves to a signed-out viewer, and it refuses protected and live videos.

YouTube'un [kullanım koşulları](https://www.youtube.com/t/terms), hizmetin açıkça izin verdiği durumlar (örneğin kendi indirme özelliği) ya da YouTube'un ve gerektiğinde hak sahiplerinin önceden yazılı izni dışında içerik indirmeye izin vermez. Bir indirmenin serbest olup olmadığı sizinle YouTube ve içerik sahipleri arasındadır; YouTube'un koşullarına uymak sizin sorumluluğunuzdadır.

## Licenses

Downpipe is licensed under Apache-2.0. Dependencies keep their own licenses; see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). Projects studied for design ideas (IDM's public documentation, aria2, yt-dlp, Gopeed, AB Download Manager, Motrix) were used as references only; no code was copied from them. FFmpeg is not bundled. Its builds are LGPL or GPL depending on the codecs enabled, and large; shipping one would need a license review and a source offer for the exact build. Downpipe instead uses an FFmpeg the user installed (on `PATH`, or the path set under **Settings → Media**) as a separate process, only to copy streams into one file without re-encoding, and saves the tracks separately when there is none. yt-dlp (public domain, [Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE)) and Deno ([MIT](https://github.com/denoland/deno/blob/main/LICENSE.md)) are not bundled either. Downpipe uses copies the user installed, or downloads the official builds from the projects' GitHub releases into its tools folder when the user presses the button for it in **Settings → Media**; those copies come from upstream under upstream's licenses (yt-dlp's standalone builds include Python and other components, listed in its release). Both run as separate processes. Other optional tools that may be integrated later (aria2, N_m3u8DL-RE) will also be invoked as separate processes, and their licenses and redistribution terms will be reviewed before any of them is bundled with a release.
