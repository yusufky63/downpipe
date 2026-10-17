# Legal notes

## Intended use

Downpipe is intended for downloading files and media that you own, have permission to download, that are made downloadable by the relevant service, or that may otherwise be lawfully downloaded. The project does not provide DRM circumvention and does not guarantee that use with a supported website is permitted by that website's terms.

Downpipe; sahibi olduğunuz, indirme izniniz bulunan, ilgili hizmetin indirilebilir olarak sunduğu ya da başka bir şekilde yasal olarak indirilebilen dosya ve medyaları indirmek için tasarlanmıştır. Proje DRM korumasını aşma imkânı sunmaz ve desteklenen bir sitede kullanımın o sitenin kullanım koşullarınca serbest olduğunu garanti etmez.

## What the project will not build

- DRM, encryption or paywall circumvention. Protected media is reported as unsupported.
- Ways around anti-bot or other security measures.
- Silent collection of browser cookies or credentials.
- A central server that downloads on users' behalf. Downpipe is local-first: links, cookies and files stay on the user's machine.

## Platform terms

That a site extractor works technically does not mean a platform allows downloading. Several platforms restrict downloading or automated access in their terms, including YouTube, X and Meta. Before the planned site providers ship, each provider's current terms will be reviewed and linked from the app, and the documentation will not claim that content from those platforms can be downloaded freely.

## Licenses

Downpipe is licensed under Apache-2.0. Dependencies keep their own licenses; see [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). Projects studied for design ideas (IDM's public documentation, aria2, yt-dlp, Gopeed, AB Download Manager, Motrix) were used as references only; no code was copied from them. Optional external tools that may be integrated later (yt-dlp, FFmpeg, aria2, N_m3u8DL-RE) will be invoked as separate processes, and their licenses and redistribution terms will be reviewed before any of them is bundled with a release.
