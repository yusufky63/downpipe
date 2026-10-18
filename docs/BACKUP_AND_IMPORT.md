# Backup and import

**Settings → App** has two sections for moving a Downpipe setup around: **Backup and import**, for Downpipe's own settings and list, and **Import from Internet Download Manager**, for IDM's list. Both only add: nothing in the list is removed or changed, a link already in the list is skipped, and no file on disk is written or replaced.

## Backup

**Export settings and list…** saves one JSON file (`downpipe-backup-<date>.json`) with:

- the settings, queues and categories included, except the paths of the programs Downpipe runs (FFmpeg, yt-dlp, Deno), which belong to one machine;
- every download in the list, in list order: its link, folder, file name, state, size and progress, queue, category, a stream's or video page's kind and chosen quality, a checksum if one was given, and when it was added and finished.

It never contains headers or cookies: the catalog does not store them (headers given with a download live in memory only) and links are stored without `user:password@`. A link's own query string is kept as it is, since it is part of the address; treat a backup like the list itself.

```json
{
  "format": "downpipe-backup",
  "version": 1,
  "app_version": "0.5.0",
  "exported_at": 1760781600,
  "settings": { "download_dir": "C:\\Users\\me\\Downloads", "max_concurrent_downloads": 3, "queues": [...], "categories": [...], ... },
  "downloads": [
    { "url": "https://example.org/files/report.pdf", "dir": "C:\\Users\\me\\Downloads\\Documents",
      "file_name": "report.pdf", "state": "completed", "total": 1048576, "downloaded": 1048576,
      "queue": "main", "category": "Documents", "auto_dir": true, "created_at": 1760780000, "completed_at": 1760780100 }
  ]
}
```

**Import…** opens a file, checks it and shows what it holds before anything changes: when and by which version it was made, how many of its downloads are new, and how many queues and categories it has. Two checkboxes choose what to take:

- **Settings.** The file's settings replace the current ones. Queues (by id) and categories (by name) from the file replace the same ones here and are added otherwise; queues and categories only this installation has stay. The FFmpeg, yt-dlp and Deno paths stay as they are, a download folder that is not an absolute path on this system keeps the current one, a category folder written for another system (`C:\...` on Linux, `/home/...` on Windows) becomes the category's folder inside the download folder, and a proxy or user agent this version refuses keeps the current one. Importing settings closes the Settings dialog, which would otherwise still show the old values.
- **Downloads.** Each download is checked on its own; one that fails is left out and counted, the rest are imported:
  - only `http` and `https` links up to 8 KiB; credentials are removed and video pages are stored under their canonical link, as when they are added;
  - a file name of at most 255 characters, made safe like a server's;
  - a folder that is an absolute path on this system, otherwise the folder its category (or the download folder) gives;
  - the state: finished and canceled downloads keep it (a finished one points at its file, whether that file is still there or not); everything else is added **paused**, so nothing starts by itself;
  - a checksum only if it parses, a queue only if it exists after the settings are taken (otherwise the main queue), short single-line labels for the category and quality; a video page's site is taken from its link, not from the file.

A link already in the list, or twice in the file, is skipped. Importing writes only to Downpipe's catalog. A paused download whose name is taken in its folder saves under a free name (`name (1).ext`) when it is resumed; the engine never overwrites a file.

Files over 64 MB, with more than 100,000 downloads, of another `format`, or from a newer `version` are refused with a message saying which.

## Import from Internet Download Manager

IDM's data is only read. Downpipe never writes to IDM's files or registry keys, and it does not need IDM to be closed.

**Read IDM's list on this computer** (Windows) opens `HKEY_CURRENT_USER\Software\DownloadManager` for reading only (`KEY_READ`) and looks at its numbered subkeys, one per download. From each it reads exactly these values:

| Value | Type | Used for |
|---|---|---|
| `Url0` | text | The link. Entries without an `http` or `https` link are skipped. |
| `FileName` | text | The name IDM shows, unless it is really the tail of a link (it contains `%`, `&`, `=` or `?`, as when a server sent a broken `Content-Disposition`). |
| `LocalPath`, `LocalFileName` | text | IDM's working folder and file. IDM downloads into its own `...\IDM\DwnlData\<user>\` folder and moves the file to its save folder when it is complete, and that save folder is not kept in a readable value; so these give the download's folder only when they lie outside `DwnlData`, and a name when `FileName` has no extension and the working file has one. |
| `FileSize` | 8 bytes, little-endian | The size, shown in the preview and kept as the download's size. `-1` (unknown) is ignored. |
| `Downloaded` | 8 bytes, little-endian | Whether IDM finished it (`Downloaded` reached `FileSize`). |

A download without a usable folder goes to the folder its category gives in Downpipe (**Folder by category** in the preview), and one without a usable name is named by the server when it starts, as any new download is.

Other values, such as `Cookie`, `User`, `EncPassword`, `CstmHdrs`, `Referer` and `UA`, and IDM's other keys (`Passwords`, `Queue`, `Scheduler`, ...), are never queried.

**Open an exported list…** reads a list exported from IDM (**Tasks → Export**), up to 32 MB, in UTF-8 or UTF-16:

- **To text file:** one link per line; lines that are not `http` or `https` links are skipped.
- **To IDM export file (`.ef2`):** one block per download, `<`, the link, header lines, `>`. The link and a `filename:` line are read; `referer:`, `User-Agent:`, `cookie:`, `postdata:` and every other line are skipped without being kept.

The preview lists every download found with its name (or the link's last part), link, folder (or **Folder by category**) and size, marked **Finished in IDM** or **In Downpipe** (already in the list; it cannot be ticked). Downloads IDM has not finished start ticked; **Select all** ticks every download Downpipe does not have yet. **Add N paused** adds the ticked ones paused, through the same checks as a backup's downloads, so nothing starts until you resume it. A finished file IDM left in its folder stays untouched: resuming saves next to it under a free name.
