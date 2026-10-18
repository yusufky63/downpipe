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

## BitTorrent

BitTorrent is a way to share files between people, and much of what is shared over it (Linux distributions, open data, public-domain films, your own files) may be shared freely. Much else may not. Download and seed only what you have the right to share: while a torrent downloads, and while it seeds afterwards, Downpipe uploads its pieces to other people, which in many countries counts as making the work available, whoever made the torrent. The other peers see your IP address (see [SECURITY_MODEL.md](SECURITY_MODEL.md#bittorrent)), and rights holders monitor public torrents for it. Downpipe does not search for torrents, ships no tracker lists and does not hide your address.

BitTorrent, paylaşma hakkınız olan içerik için kullanılmalıdır. Bir torrent inerken ve sonrasında paylaşılırken (seed) Downpipe parçalarını başka kişilere gönderir; birçok ülkede bu, eseri erişime açmak sayılır. Diğer eşler IP adresinizi görür. Yalnızca paylaşma hakkınız olan içeriği indirin ve paylaşın.

Seeding is on by default with conservative limits (until the uploaded amount equals the downloaded one, ratio 1.0, or two hours, whichever comes first, and never after Downpipe quits), and **Settings → BitTorrent** can turn it off entirely.

## Licenses

Downpipe is licensed under Apache-2.0. Dependencies keep their own licenses; see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). BitTorrent runs in the app through [librqbit](https://github.com/ikatson/rqbit) (Apache-2.0), compiled in, not a separate program. Projects studied for design ideas (IDM's public documentation, aria2, yt-dlp, Gopeed, AB Download Manager, Motrix) were used as references only; no code was copied from them. FFmpeg is not bundled. Its builds are LGPL or GPL depending on the codecs enabled, and large; shipping one would need a license review and a source offer for the exact build. Downpipe runs FFmpeg as a separate process, only to copy streams into one file without re-encoding, and saves the tracks separately when there is none. It uses an FFmpeg the user installed (on `PATH`, or the path set under **Settings → Media**), or, when the user presses a download button, fetches the static `lgpl` build of [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) for Windows or Linux (x86-64 and arm64) from its GitHub release into the tools folder. That build is LGPL-3.0 (its archive's `LICENSE.txt`) and leaves out GPL-only libraries such as x264 and x265, so no GPL terms come with it; the copy comes from upstream under upstream's license, like yt-dlp and Deno below, and Downpipe does not redistribute it. Only `ffmpeg` and `ffprobe` are taken from the archive. macOS has no FFmpeg build that publishes checksums (evermeet.cx publishes GPG signatures of GPL builds with non-free codecs, other sites publish hashes only on web pages), so there the user installs FFmpeg, for example with Homebrew. yt-dlp (public domain, [Unlicense](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE)) and Deno ([MIT](https://github.com/denoland/deno/blob/main/LICENSE.md)) are not bundled either. Downpipe uses copies the user installed, or downloads the official builds from the projects' GitHub releases into its tools folder when the user presses a button for it (in **Settings → Media**, or in a dialog that needs a missing tool), and replaces the yt-dlp it downloaded with newer releases once a day unless that is turned off; those copies come from upstream under upstream's licenses (yt-dlp's standalone builds include Python and other components, listed in its release). Both run as separate processes. Other optional tools that may be integrated later (aria2, N_m3u8DL-RE) will also be invoked as separate processes, and their licenses and redistribution terms will be reviewed before any of them is bundled with a release.
