import { describe, expect, it } from "vitest";
import {
  DEFAULT_MEDIA_MIN_BYTES,
  DEFAULT_MEDIA_TYPES,
  MAX_MEDIA_TYPES,
  MAX_TAB_MEDIA,
  MAX_TAB_PARTS,
  addMedia,
  classifyResponse,
  classifySource,
  cleanTitle,
  defaultMediaFilters,
  excludeMatchesFor,
  frameOf,
  isPartOf,
  kindLabel,
  mediaForVideo,
  mediaExtension,
  mediaLabels,
  mediaName,
  pageHeaders,
  parseMediaFilters,
  posterIds,
  parseMediaTypeList,
  parsePlaylist,
  parseTabMedia,
  parseTabParts,
  playlistKey,
  readPlaylist,
  readPlaylistText,
  responseSize,
  shouldReadPlaylist,
  tabMediaKey,
  tabPartsKey,
  titleFileName,
  validMediaFilters,
  withoutRangeParams,
  type PlaylistInfo,
  type ResponseLike,
  type TabMedia,
} from "./media";

function response(patch: Partial<ResponseLike> & { headers?: Record<string, string> }): ResponseLike {
  const { headers = {}, ...rest } = patch;
  return {
    url: "https://cdn.example.net/v/file.bin",
    type: "xmlhttprequest",
    statusCode: 200,
    tabId: 7,
    responseHeaders: Object.entries(headers).map(([name, value]) => ({ name, value })),
    ...rest,
  };
}

describe("classifyResponse", () => {
  it("finds HLS playlists by extension or type, from any request type", () => {
    const url = "https://v.example.net/a/master.m3u8?sig=1";
    expect(classifyResponse(response({ url }))).toEqual({ kind: "hls", url });
    for (const type of ["application/vnd.apple.mpegURL", "application/mpegurl", "audio/x-mpegurl"]) {
      expect(classifyResponse(response({ url: "https://v.example.net/play", headers: { "Content-Type": type } }))?.kind).toBe("hls");
    }
    expect(classifyResponse(response({ url: "https://v.example.net/a/index.m3u8", type: "media" }))?.kind).toBe("hls");
  });

  it("leaves YouTube's video servers to the page entry", () => {
    const url = "https://rr1---sn-4g5e6nsz.googlevideo.com/videoplayback?expire=1&mime=video%2Fmp4&range=0-100000";
    expect(classifyResponse(response({ url, type: "media", headers: { "Content-Type": "video/mp4" } }))).toBeNull();
    expect(
      classifyResponse(response({ url: "https://manifest.googlevideo.com/api/manifest/hls_playlist/id/1/index.m3u8" })),
    ).toBeNull();
  });

  it("finds DASH manifests", () => {
    expect(classifyResponse(response({ url: "https://v.example.net/a/manifest.mpd" }))?.kind).toBe("dash");
    expect(
      classifyResponse(response({ url: "https://v.example.net/m", headers: { "content-type": "application/dash+xml; charset=utf-8" } }))
        ?.kind,
    ).toBe("dash");
  });

  it("finds media files that a media element loaded, with their size", () => {
    expect(
      classifyResponse(
        response({
          type: "media",
          statusCode: 206,
          url: "https://v.example.net/clip.mp4",
          headers: { "Content-Type": "video/mp4", "Content-Range": "bytes 0-1023/52428800", "Content-Length": "1024" },
        }),
      ),
    ).toEqual({ kind: "direct", mime: "video/mp4", size: 52428800, url: "https://v.example.net/clip.mp4" });
    expect(
      classifyResponse(response({ type: "media", url: "https://v.example.net/song.mp3", headers: { "content-type": "application/octet-stream" } })),
    ).toEqual({ kind: "direct", mime: undefined, size: undefined, url: "https://v.example.net/song.mp3" });
    expect(
      classifyResponse(response({ type: "media", url: "https://v.example.net/a.ogg", headers: { "content-type": "application/ogg" } }))?.mime,
    ).toBe("application/ogg");
  });

  it("finds whole files a script fetched in pieces", () => {
    // Facebook and Instagram style: the range is in the query string.
    expect(
      classifyResponse(
        response({
          url: "https://video.cdn.example/v/clip.mp4?efg=1&bytestart=0&byteend=1023",
          headers: { "Content-Type": "video/mp4", "Content-Length": "1024" },
        }),
      ),
    ).toEqual({ kind: "direct", mime: "video/mp4", size: undefined, url: "https://video.cdn.example/v/clip.mp4?efg=1" });
    // A range answer naming the file's total size.
    expect(
      classifyResponse(
        response({
          statusCode: 206,
          url: "https://v.example.net/track-720.mp4",
          headers: { "Content-Type": "video/mp4", "Content-Range": "bytes 1000-2000/9000000" },
        }),
      ),
    ).toMatchObject({ kind: "direct", size: 9000000 });
  });

  it("leaves out fragments, small files, page requests and errors", () => {
    const cases: ResponseLike[] = [
      // A player fetching pieces of a stream itself.
      response({ url: "https://v.example.net/seg-1.mp4", headers: { "Content-Type": "video/mp4" } }),
      response({ type: "media", url: "https://v.example.net/seg-1.ts", headers: { "Content-Type": "video/mp2t" } }),
      response({ type: "media", url: "https://v.example.net/seg-1.m4s", headers: { "Content-Type": "video/mp4" } }),
      response({ type: "media", url: "https://v.example.net/beep.mp3", headers: { "Content-Type": "audio/mpeg", "Content-Length": "4000" } }),
      response({ type: "media", url: "https://v.example.net/page", headers: { "Content-Type": "text/html" } }),
      response({ url: "https://v.example.net/a.m3u8", statusCode: 404 }),
      response({ url: "https://v.example.net/a.m3u8", tabId: -1 }),
      response({ url: "blob:https://v.example.net/0f1e" }),
      response({ url: `https://v.example.net/${"a".repeat(9000)}.m3u8` }),
    ];
    for (const r of cases) expect(classifyResponse(r), r.url.slice(0, 60)).toBeNull();
    expect(
      classifyResponse(
        response({ type: "media", url: "https://v.example.net/clip.webm", headers: { "Content-Length": String(DEFAULT_MEDIA_MIN_BYTES) } }),
      )?.kind,
    ).toBe("direct");
  });

  it("follows the user's types and minimum size", () => {
    const flv = response({ type: "media", url: "https://v.example.net/old.flv", headers: { "Content-Type": "video/x-flv" } });
    expect(classifyResponse(flv)?.kind).toBe("direct");
    const onlyAudio = { ...defaultMediaFilters(), mediaTypes: ["mp3", "m4a"] };
    expect(classifyResponse(flv, onlyAudio)).toBeNull();
    // Streams are always listed.
    expect(classifyResponse(response({ url: "https://v.example.net/m.mpd" }), { ...onlyAudio, mediaTypes: [] })?.kind).toBe("dash");
    const small = response({ type: "media", url: "https://v.example.net/beep.mp3", headers: { "Content-Length": "4000" } });
    expect(classifyResponse(small, { ...defaultMediaFilters(), mediaMinBytes: 0 })?.kind).toBe("direct");
    // A file without an extension goes by its type.
    const typed = response({ type: "media", url: "https://v.example.net/stream?id=4", headers: { "Content-Type": "audio/mpeg" } });
    expect(classifyResponse(typed, onlyAudio)?.kind).toBe("direct");
    expect(classifyResponse(typed, { ...onlyAudio, mediaTypes: ["m4a"] })).toBeNull();
  });
});

describe("helpers", () => {
  it("reads the whole file's size", () => {
    expect(responseSize(response({ statusCode: 206, headers: { "content-range": "bytes 0-1/99", "content-length": "2" } }))).toBe(99);
    expect(responseSize(response({ headers: { "content-length": "42" } }))).toBe(42);
    expect(responseSize(response({ statusCode: 206, headers: { "content-length": "2" } }))).toBeUndefined();
    expect(responseSize(response({ headers: { "content-length": "lots" } }))).toBeUndefined();
  });

  it("drops range query parameters only", () => {
    expect(withoutRangeParams("https://a.org/v.mp4?bytestart=0&byteend=9&x=1")).toBe("https://a.org/v.mp4?x=1");
    expect(withoutRangeParams("https://a.org/v.mp4?range=0-9")).toBe("https://a.org/v.mp4");
    expect(withoutRangeParams("https://a.org/v.mp4?x=1")).toBeNull();
  });

  it("names a file's type from its URL or its MIME type", () => {
    expect(mediaExtension("https://a.org/v/clip.MOV", undefined)).toBe("mov");
    expect(mediaExtension("https://a.org/v/clip", "audio/webm")).toBe("weba");
    expect(mediaExtension("https://a.org/v/page.php", "video/mp4")).toBe("mp4");
    expect(mediaExtension("https://a.org/v/x", "video/unknown")).toBeUndefined();
  });

  it("keeps the page's Referer and Origin, nothing else", () => {
    expect(
      pageHeaders([
        { name: "Referer", value: "https://user:pw@x.com/status/1#top" },
        { name: "Origin", value: "https://x.com" },
        { name: "Cookie", value: "session=1" },
      ]),
    ).toEqual({ referrer: "https://x.com/status/1", origin: "https://x.com" });
    expect(pageHeaders([{ name: "origin", value: "null" }])).toEqual({ referrer: undefined, origin: undefined });
    expect(pageHeaders(undefined)).toEqual({ referrer: undefined, origin: undefined });
  });

  it("keeps the overlay off excluded sites", () => {
    expect(excludeMatchesFor(["example.com", "*.example.org", "[::1]"])).toEqual(["*://example.com/*", "*://*.example.org/*"]);
  });
});

describe("media settings", () => {
  it("default to an IDM-like list and a minimum size", () => {
    expect(parseMediaFilters({})).toEqual({
      mediaOverlay: true,
      overlayHoverOnly: false,
      overlaySites: [],
      mediaTypes: [...DEFAULT_MEDIA_TYPES],
      mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES,
    });
    expect(
      parseMediaFilters({
        mediaOverlay: false,
        overlayHoverOnly: true,
        overlaySites: ["Example.COM", "https://news.example.org/a", "not a site", "example.com", 4],
        mediaTypes: ["MP4", ".mkv", "bad type", 3],
        mediaMinBytes: -1,
      }),
    ).toEqual({
      mediaOverlay: false,
      overlayHoverOnly: true,
      overlaySites: ["example.com", "news.example.org"],
      mediaTypes: ["mp4", "mkv"],
      mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES,
    });
  });

  it("accept only stored forms from the options page", () => {
    const ok = { mediaOverlay: true, overlayHoverOnly: false, overlaySites: ["*.example.com"], mediaTypes: ["mp4"], mediaMinBytes: 0 };
    expect(validMediaFilters(ok)).toEqual(ok);
    for (const bad of [
      { ...ok, mediaTypes: ["MP4"] },
      { ...ok, mediaTypes: ["mp4", "mp4"] },
      { ...ok, mediaOverlay: "yes" },
      { ...ok, overlayHoverOnly: 1 },
      { ...ok, overlaySites: ["Example.com"] },
      { ...ok, overlaySites: ["a.org", "a.org"] },
      { ...ok, mediaMinBytes: 1.5 },
      { ...ok, extra: 1 },
      { mediaOverlay: true, mediaTypes: ["mp4"], mediaMinBytes: 0 },
      null,
    ]) {
      expect(validMediaFilters(bad), JSON.stringify(bad)).toBeNull();
    }
  });

  it("parse the type field, where an empty list is fine", () => {
    expect(parseMediaTypeList("mp4, .MKV  weba")).toEqual({ values: ["mp4", "mkv", "weba"], errors: [] });
    expect(parseMediaTypeList("")).toEqual({ values: [], errors: [] });
    expect(parseMediaTypeList("mp4 not/a/type").errors).toHaveLength(1);
    const many = Array.from({ length: MAX_MEDIA_TYPES + 1 }, (_, i) => `t${i}`).join(" ");
    expect(parseMediaTypeList(many).errors).toEqual([`List at most ${MAX_MEDIA_TYPES} file types.`]);
  });
});

const base = { kind: "hls" as const };
const empty: TabMedia = { items: [], parts: [] };

/** Adds each in turn, as the service worker does; `null` playlist info means it could not be read. */
function play(steps: [url: string, playlist?: PlaylistInfo | null][], start: TabMedia = empty): TabMedia {
  return steps.reduce(
    (tab, [url, playlist], i) => addMedia(tab, { ...base, url }, `id${i}`, i, playlist) ?? tab,
    start,
  );
}

const urls = (tab: TabMedia) => tab.items.map((c) => c.url);

// X's layout, as video.twimg.com serves it (October 2026): the quality and
// audio playlists sit in folders below the master.
const X_MASTER = "https://video.twimg.com/amplify_video/2110883883948314624/pl/jSDqt-Zcg5sAxPk9.m3u8?v=1ea";
const X_BODY = `#EXTM3U
#EXT-X-VERSION:6
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-MEDIA:NAME="Audio",TYPE=AUDIO,GROUP-ID="audio-32000",AUTOSELECT=YES,URI="/amplify_video/2110883883948314624/pl/mp4a/32000/w459BU6Ge5oSDhrJ.m3u8"
#EXT-X-MEDIA:NAME="Audio",TYPE=AUDIO,GROUP-ID="audio-128000",AUTOSELECT=YES,URI="/amplify_video/2110883883948314624/pl/mp4a/128000/7e0vLpc_DftLQFBT.m3u8"

#EXT-X-STREAM-INF:AVERAGE-BANDWIDTH=82090,BANDWIDTH=136339,RESOLUTION=480x270,CODECS="mp4a.40.2,avc1.4D4015",AUDIO="audio-32000"
/amplify_video/2110883883948314624/pl/avc1/480x270/WSXbxxPdKZVvWWM4.m3u8
#EXT-X-STREAM-INF:AVERAGE-BANDWIDTH=587439,BANDWIDTH=1044799,RESOLUTION=1280x720,CODECS="mp4a.40.2,avc1.64001F",AUDIO="audio-128000"
/amplify_video/2110883883948314624/pl/avc1/1280x720/ukCKNrwZwVL2gfYS.m3u8
#EXT-X-STREAM-INF:AVERAGE-BANDWIDTH=2632011,BANDWIDTH=4458865,RESOLUTION=1920x1080,CODECS="mp4a.40.2,avc1.640032",AUDIO="audio-128000"
/amplify_video/2110883883948314624/pl/avc1/1920x1080/wa0X4bxE_xmzp8eh.m3u8
`;
const X_720 = "https://video.twimg.com/amplify_video/2110883883948314624/pl/avc1/1280x720/ukCKNrwZwVL2gfYS.m3u8";
const X_AUDIO = "https://video.twimg.com/amplify_video/2110883883948314624/pl/mp4a/128000/7e0vLpc_DftLQFBT.m3u8";

// X's broadcast replays (Periscope): the qualities sit in a sibling folder
// (`transcode/` next to `non_transcode/`), shortened here.
const P_ROOT = "https://prod-fastly-us-west-1.video.pscp.tv/Transcoding/v1/hls/080fQE";
const P_MASTER = `${P_ROOT}/non_transcode/us-west-1/public/master_dynamic_1665.m3u8?type=replay`;
const P_BODY = `#EXTM3U
#EXT-X-DYNAMICALLY-GENERATED
#EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=1200000,RESOLUTION=848x480,CODECS="avc1.64001e,mp4a.40.2"
/Transcoding/v1/hls/080fQE/transcode/us-west-1/public/eyJ480/playlist_1665.m3u8?type=replay
#EXT-X-STREAM-INF:PROGRAM-ID=1,BANDWIDTH=2750000,RESOLUTION=1280x720,CODECS="avc1.64001f,mp4a.40.2"
/Transcoding/v1/hls/080fQE/transcode/us-west-1/public/eyJ720/playlist_1665.m3u8?type=replay
`;
const P_480 = `${P_ROOT}/transcode/us-west-1/public/eyJ480/playlist_1665.m3u8?type=replay`;
const P_720 = `${P_ROOT}/transcode/us-west-1/public/eyJ720/playlist_1665.m3u8?type=replay`;

// Everything in one folder with relative URIs, like the test server's
// `/hls/flat/` stream and many CDNs.
const F_MASTER = "https://cdn.example.net/v/77/8k4_FZTUGeXpWu9v.m3u8";
const F_BODY = `#EXTM3U\r
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aud",NAME="English",URI="srcE71IRxCs1PHFJ.m3u8"\r
#EXT-X-STREAM-INF:BANDWIDTH=800000,AUDIO="aud"\r
qLow0000000000aa.m3u8?token=t#frag\r
#EXT-X-STREAM-INF:BANDWIDTH=2400000,AUDIO="aud"\r
https://cdn.example.net/v/77/qHigh000000000bb.m3u8?token=t\r
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=90000,URI="iframes.m3u8"\r
`;
const F_AUDIO = "https://cdn.example.net/v/77/srcE71IRxCs1PHFJ.m3u8";
const F_LOW = "https://cdn.example.net/v/77/qLow0000000000aa.m3u8?token=t";
const F_HIGH = "https://cdn.example.net/v/77/qHigh000000000bb.m3u8?token=t";
const MEDIA_BODY = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4.0,\nseg-0.m4s\n#EXT-X-ENDLIST\n";

const parsed = (body: string, url: string) => parsePlaylist(body, url) ?? undefined;

describe("parsePlaylist", () => {
  it("lists a master's quality and audio playlists, resolved against it", () => {
    expect(parsePlaylist(X_BODY, X_MASTER)).toEqual({
      master: true,
      parts: [
        "https://video.twimg.com/amplify_video/2110883883948314624/pl/mp4a/32000/w459BU6Ge5oSDhrJ.m3u8",
        X_AUDIO,
        "https://video.twimg.com/amplify_video/2110883883948314624/pl/avc1/480x270/WSXbxxPdKZVvWWM4.m3u8",
        X_720,
        "https://video.twimg.com/amplify_video/2110883883948314624/pl/avc1/1920x1080/wa0X4bxE_xmzp8eh.m3u8",
      ],
      qualities: 3,
    });
    expect(parsePlaylist(P_BODY, P_MASTER)).toEqual({ master: true, parts: [P_480, P_720], qualities: 2 });
  });

  it("reads relative and absolute URIs, CRLF lines, and I-frame playlists", () => {
    expect(parsePlaylist(F_BODY, F_MASTER)).toEqual({
      master: true,
      parts: [F_AUDIO, F_LOW, F_HIGH, "https://cdn.example.net/v/77/iframes.m3u8"],
      // No resolutions: each variant counts.
      qualities: 2,
    });
  });

  it("counts qualities by resolution, so codec twins are one", () => {
    const body = [
      "#EXTM3U",
      "#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION=1920x1080,CODECS=\"avc1.640028\"",
      "h264.m3u8",
      "#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION=1920x1080,CODECS=\"hvc1.2.4.L123\"",
      "hevc.m3u8",
    ].join("\n");
    expect(parsePlaylist(body, "https://a.org/m.m3u8")).toMatchObject({ qualities: 1 });
  });

  it("tells a media playlist and rejects what is not a playlist", () => {
    expect(parsePlaylist(MEDIA_BODY, "https://a.org/v/720.m3u8")).toEqual({ master: false });
    expect(parsePlaylist("\uFEFF#EXTM3U\n#EXTINF:4,\na.ts\n", "https://a.org/v.m3u8")).toEqual({ master: false });
    expect(parsePlaylist("<html>Sign in</html>", "https://a.org/v.m3u8")).toBeNull();
    expect(parsePlaylist("", "https://a.org/v.m3u8")).toBeNull();
  });

  it("leaves out URIs that are not web addresses", () => {
    const body = `#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,URI="data:text/plain,x"\n#EXT-X-STREAM-INF:BANDWIDTH=1\njavascript:alert(1)\n#EXT-X-STREAM-INF:BANDWIDTH=2\nok.m3u8\n`;
    expect(parsePlaylist(body, "https://a.org/m.m3u8")).toEqual({ master: true, parts: ["https://a.org/ok.m3u8"], qualities: 1 });
  });
});

describe("playlistKey", () => {
  it("drops the fragment and low-latency reload parameters only", () => {
    expect(playlistKey("https://a.org/v.m3u8?token=1&_HLS_msn=12&_HLS_part=3#t")).toBe("https://a.org/v.m3u8?token=1");
    expect(playlistKey("https://a.org/v.m3u8?tag=12")).toBe("https://a.org/v.m3u8?tag=12");
  });
});

/** A body that hands out one chunk per read, counting them. */
function stream(chunks: string[], seen = { pulled: 0, canceled: false }): ReadableStream<Uint8Array> {
  const encoder = new TextEncoder();
  return new ReadableStream<Uint8Array>(
    {
      pull(controller) {
        const chunk = chunks[seen.pulled++];
        if (chunk === undefined) controller.close();
        else controller.enqueue(encoder.encode(chunk));
      },
      cancel() {
        seen.canceled = true;
      },
    },
    { highWaterMark: 0 },
  );
}

describe("readPlaylistText", () => {
  it("reads a whole master", async () => {
    expect(await readPlaylistText(stream([X_BODY.slice(0, 100), X_BODY.slice(100)]))).toBe(X_BODY);
  });

  it("stops at the size cap, on a whole line", async () => {
    const seen = { pulled: 0, canceled: false };
    const text = await readPlaylistText(stream(["#EXTM3U\nline-one\n", "line-two-is-long\n", "more\n"], seen), 20);
    expect(text).toBe("#EXTM3U\nline-one\n");
    expect(seen).toEqual({ pulled: 2, canceled: true });
  });

  it("stops at the first segment of a media playlist", async () => {
    const seen = { pulled: 0, canceled: false };
    const chunks = ["#EXTM3U\n#EXT-X-TARGETDURATION:4\n", "#EXTINF:4,\nseg-0.ts\n", "#EXTINF:4,\nseg-1.ts\n"];
    expect(parsePlaylist(await readPlaylistText(stream(chunks, seen)), "https://a.org/v.m3u8")).toEqual({ master: false });
    expect(seen).toEqual({ pulled: 2, canceled: true });
  });
});

describe("readPlaylist", () => {
  it("fetches without cookies or referrer and parses against the final address", async () => {
    let seen: RequestInit | undefined;
    const fetchFn = (url: string, init: RequestInit) => {
      seen = init;
      expect(url).toBe(F_MASTER);
      // Redirected to another server: the relative URIs point there.
      const response = new Response(F_BODY);
      Object.defineProperty(response, "url", { value: "https://edge.example.net/v/77/master.m3u8" });
      return Promise.resolve(response);
    };
    const info = await readPlaylist(fetchFn, F_MASTER);
    expect(info).toMatchObject({ master: true, qualities: 2 });
    expect(info?.master && info.parts[0]).toBe("https://edge.example.net/v/77/srcE71IRxCs1PHFJ.m3u8");
    expect(seen).toMatchObject({ credentials: "omit", referrerPolicy: "no-referrer" });
  });

  it("gives up on errors, refusals and slow servers", async () => {
    expect(await readPlaylist(() => Promise.reject(new TypeError("offline")), F_MASTER)).toBeNull();
    expect(await readPlaylist(() => Promise.resolve(new Response("no", { status: 403 })), F_MASTER)).toBeNull();
    const hang = (_url: string, init: RequestInit) =>
      new Promise<Response>((_, reject) => init.signal?.addEventListener("abort", () => reject(new Error("aborted"))));
    expect(await readPlaylist(hang, F_MASTER, 20)).toBeNull();
  });
});

describe("addMedia", () => {
  it("adds newest first and ignores repeats", () => {
    let tab = empty;
    tab = addMedia(tab, { ...base, url: "https://v.example.net/a/master.m3u8#x", referrer: undefined }, "a", 1) ?? tab;
    expect(tab.items).toEqual([{ ...base, url: "https://v.example.net/a/master.m3u8", id: "a", at: 1 }]);
    expect(addMedia(tab, { ...base, url: "https://v.example.net/a/master.m3u8" }, "b", 2)).toBeNull();
    const next = addMedia(tab, { kind: "direct", url: "https://v.example.net/b.mp4", size: 9, origin: "https://p.example" }, "d", 4);
    expect(next?.items.map((c) => c.id)).toEqual(["d", "a"]);
    expect(next?.items[0]?.origin).toBe("https://p.example");
    expect(next?.parts).toBe(tab.parts);
  });

  it("hides playlists in folders below a listed one (X), even unread", () => {
    expect(urls(play([[X_MASTER], [X_720], [X_AUDIO]]))).toEqual([X_MASTER]);
    // And the other way round: a quality playlist the player fetched first.
    expect(urls(play([[X_720], [X_AUDIO], [X_MASTER, null]]))).toEqual([X_MASTER]);
  });

  it("lists only the master of a stream whose parts sit beside it", () => {
    const master = parsed(F_BODY, F_MASTER);
    const tab = play([[F_MASTER, master], [F_LOW], [F_AUDIO], [F_HIGH]]);
    expect(urls(tab)).toEqual([F_MASTER]);
    expect(tab.items[0]?.qualities).toBe(2);
    expect(tab.parts).toContain(F_AUDIO);
    // Periscope: parts in a sibling folder.
    expect(urls(play([[P_MASTER, parsed(P_BODY, P_MASTER)], [P_480], [P_720]]))).toEqual([P_MASTER]);
  });

  it("replaces quality playlists listed before their master shows up", () => {
    const media = { master: false } as const;
    const before = play([[F_LOW, media], [F_AUDIO, media], [P_720, media]]);
    expect(before.items).toHaveLength(3);
    const after = play([[F_MASTER, parsed(F_BODY, F_MASTER)]], before);
    expect(urls(after)).toEqual([F_MASTER, P_720]);
    // Remembered after the master has left the list too.
    const later = { items: [], parts: after.parts };
    expect(addMedia(later, { ...base, url: `${F_HIGH}#t` }, "x", 9)).toBeNull();
  });

  it("keeps masters of different videos, even in one folder", () => {
    const other = "https://cdn.example.net/v/77/0therMaster00000.m3u8";
    const otherBody = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1,RESOLUTION=640x360\nlow360.m3u8\n";
    const tab = play([
      [F_MASTER, parsed(F_BODY, F_MASTER)],
      [other, parsed(otherBody, other)],
    ]);
    expect(urls(tab)).toEqual([other, F_MASTER]);
    // A master in a folder below another is still a master once read.
    const deeper = "https://cdn.example.net/v/77/clips/master.m3u8";
    expect(urls(play([[deeper, parsed(otherBody, deeper)]], tab))).toEqual([deeper, other, F_MASTER]);
  });

  it("lets a media playlist that was read hide nothing", () => {
    const top = "https://v.example.net/a/index.m3u8";
    const listed = play([["https://v.example.net/a/hd/index.m3u8", { master: false }]]);
    expect(play([[top, { master: false }]], listed).items).toHaveLength(2);
  });

  it("keeps playlists from other folders and sites apart", () => {
    const tab = play([["https://v.example.net/a/master.m3u8"]]);
    expect(addMedia(tab, { ...base, url: "https://v.example.net/a/other.m3u8" }, "b", 2)?.items).toHaveLength(2);
    expect(addMedia(tab, { ...base, url: "https://w.example.net/a/hd/index.m3u8" }, "c", 2)?.items).toHaveLength(2);
  });

  it("treats low-latency reloads of a playlist as the same playlist", () => {
    const tab = play([["https://v.example.net/live/720.m3u8"]]);
    expect(addMedia(tab, { ...base, url: "https://v.example.net/live/720.m3u8?_HLS_msn=40&_HLS_part=1" }, "b", 2)).toBeNull();
  });

  it("keeps the list and the parts bounded", () => {
    let tab = empty;
    for (let i = 0; i < MAX_TAB_MEDIA + 5; i++) {
      tab = addMedia(tab, { kind: "direct", url: `https://v.example.net/${i}.mp4` }, `id${i}`, i) ?? tab;
    }
    expect(tab.items).toHaveLength(MAX_TAB_MEDIA);
    expect(tab.items[0]?.id).toBe(`id${MAX_TAB_MEDIA + 4}`);
    const many: PlaylistInfo = {
      master: true,
      parts: Array.from({ length: MAX_TAB_PARTS }, (_, i) => `https://v.example.net/p/${i}.m3u8`),
      qualities: 1,
    };
    const full = play([["https://v.example.net/m1.m3u8", many]]);
    const more = play([["https://v.example.net/m2.m3u8", { master: true, parts: ["https://v.example.net/new.m3u8"], qualities: 1 }]], full);
    expect(more.parts).toHaveLength(MAX_TAB_PARTS);
    expect(more.parts[0]).toBe("https://v.example.net/new.m3u8");
  });
});

describe("shouldReadPlaylist", () => {
  it("reads only playlists that are new to the tab", () => {
    const tab = play([[F_MASTER, parsed(F_BODY, F_MASTER)], [X_MASTER]]);
    expect(shouldReadPlaylist(tab, P_MASTER)).toBe(true);
    expect(shouldReadPlaylist(tab, `${F_MASTER}#again`)).toBe(false);
    expect(shouldReadPlaylist(tab, F_HIGH)).toBe(false);
    // Below a listed playlist: hidden without a request.
    expect(shouldReadPlaylist(tab, X_720)).toBe(false);
  });
});

describe("isPartOf", () => {
  it("needs a deeper folder on the same origin", () => {
    const master = "https://video.twimg.com/amplify_video/1/pl/abc.m3u8?tag=16";
    expect(isPartOf("https://video.twimg.com/amplify_video/1/pl/avc1/1280x720/x.m3u8", master)).toBe(true);
    expect(isPartOf("https://video.twimg.com/amplify_video/1/pl/def.m3u8", master)).toBe(false);
    expect(isPartOf("https://video.twimg.com/amplify_video/2/pl/avc1/x.m3u8", master)).toBe(false);
    expect(isPartOf("not a url", master)).toBe(false);
  });
});

describe("parseTabMedia", () => {
  it("keeps only well-formed entries", () => {
    const good = {
      id: "a-1",
      kind: "dash",
      url: "https://v.example.net/m.mpd",
      at: 1,
      frameUrl: "https://example.org/",
      referrer: "https://example.org/watch",
      origin: "https://example.org",
    };
    expect(
      parseTabMedia([
        good,
        { ...good, id: "bad id" },
        { ...good, kind: "rtmp" },
        { ...good, url: "javascript:alert(1)" },
        { ...good, frameUrl: "file:///etc" },
        { ...good, referrer: "about:blank" },
        { ...good, origin: 3 },
        { ...good, size: -1 },
        { ...good, qualities: 1.5 },
        null,
      ]),
    ).toEqual([good]);
    expect(parseTabMedia([{ ...good, qualities: 3 }])).toEqual([{ ...good, qualities: 3 }]);
    expect(parseTabMedia("nope")).toEqual([]);
    expect(tabMediaKey(12)).toBe("media:12");
  });

  it("keeps a tab's playlist parts under the same prefix", () => {
    expect(parseTabParts(["https://a.org/1.m3u8", "file:///x.m3u8", 3, "https://a.org/2.m3u8"])).toEqual([
      "https://a.org/1.m3u8",
      "https://a.org/2.m3u8",
    ]);
    expect(parseTabParts({})).toEqual([]);
    expect(tabPartsKey(12).startsWith(tabMediaKey(12))).toBe(true);
  });
});

describe("labels and names", () => {
  it("names the kind of media", () => {
    expect(kindLabel({ kind: "hls", url: "https://a.org/x.m3u8" })).toBe("HLS stream");
    expect(kindLabel({ kind: "hls", url: "https://a.org/x.m3u8", qualities: 3 })).toBe("HLS · 3 qualities");
    expect(kindLabel({ kind: "hls", url: "https://a.org/x.m3u8", qualities: 1 })).toBe("HLS · 1 quality");
    // A master with only audio renditions.
    expect(kindLabel({ kind: "hls", url: "https://a.org/x.m3u8", qualities: 0 })).toBe("HLS stream");
    expect(kindLabel({ kind: "dash", url: "https://a.org/x.mpd" })).toBe("DASH stream");
    expect(kindLabel({ kind: "direct", url: "https://a.org/x.mp4", mime: "video/mp4" })).toBe("MP4 video");
    expect(kindLabel({ kind: "direct", url: "https://a.org/x.mp3" })).toBe("MP3 audio");
    expect(kindLabel({ kind: "direct", url: "https://a.org/x", mime: "audio/webm" })).toBe("WEBA audio");
    expect(kindLabel({ kind: "direct", url: "https://a.org/x", mime: "audio/x-matroska-long" })).toBe("Audio");
  });

  it("shows the file name from the URL", () => {
    expect(mediaName("https://a.org/v/My%20clip.mp4?x=1")).toBe("My clip.mp4");
    expect(mediaName("https://a.org/")).toBe("a.org");
  });

  it("names rows after the page, numbered from the oldest when several share it", () => {
    const one = [{ url: "https://video.twimg.com/a/pl/8k4_FZTUGeXpWu9v.m3u8" }];
    expect(mediaLabels(one, "  Home / X ")).toEqual(["Home / X"]);
    // Newest first, as the list is kept.
    const three = [{ url: "https://a.org/c.m3u8" }, { url: "https://a.org/b.mp4" }, { url: "https://a.org/a.m3u8" }];
    expect(mediaLabels(three, "Home / X")).toEqual(["Home / X (3)", "Home / X (2)", "Home / X"]);
    // Without a title, the file names.
    expect(mediaLabels(one, undefined)).toEqual(["8k4_FZTUGeXpWu9v.m3u8"]);
    expect(mediaLabels(one, "\u0007")).toEqual(["8k4_FZTUGeXpWu9v.m3u8"]);
    expect(mediaLabels([], "Home / X")).toEqual([]);
  });

  it("cleans page titles", () => {
    expect(cleanTitle("  A\tvideo\n title  ")).toBe("A video title");
    expect(cleanTitle("\u0000\u0007")).toBeUndefined();
    expect(cleanTitle(undefined)).toBeUndefined();
    // A bare site name, as X's tab shows before the post loads, names nothing.
    expect(cleanTitle("X")).toBeUndefined();
    expect(cleanTitle(" ab ")).toBeUndefined();
    expect(cleanTitle("Zoo")).toBe("Zoo");
    expect([...(cleanTitle("x".repeat(500)) ?? "")]).toHaveLength(200);
  });

  it("names direct files after the page", () => {
    expect(titleFileName('Cats: "the movie"?', { url: "https://a.org/v/123.mp4" })).toBe("Cats the movie.mp4");
    expect(titleFileName("Song", { url: "https://a.org/stream?id=1", mime: "audio/mpeg" })).toBe("Song.mp3");
    expect(titleFileName("Clip", { url: "https://a.org/stream", mime: "video/webm" })).toBe("Clip.webm");
    expect(titleFileName(undefined, { url: "https://a.org/v.mp4" })).toBeUndefined();
    expect(titleFileName("Clip", { url: "https://a.org/stream" })).toBeUndefined();
  });
});

describe("sources and frames", () => {
  it("classifies an element's own source", () => {
    expect(classifySource("https://a.org/v/index.m3u8")).toEqual({ kind: "hls" });
    expect(classifySource("https://a.org/v/manifest.mpd")).toEqual({ kind: "dash" });
    expect(classifySource("https://a.org/v/clip.mp4")).toEqual({ kind: "direct" });
    expect(classifySource("blob:https://a.org/1")).toBeNull();
  });

  it("takes the frame from Firefox's document URL or Chrome's initiator", () => {
    expect(frameOf({ documentUrl: "https://user:pw@player.example/embed/1#t=3" })).toBe("https://player.example/embed/1");
    expect(frameOf({ initiator: "https://player.example" })).toBe("https://player.example/");
    expect(frameOf({ initiator: "null" })).toBeUndefined();
    expect(frameOf({})).toBeUndefined();
  });
});

// What an X post with two videos loads (seen on a real post, signed out).
const X_POST = "https://pbs.twimg.com/amplify_video_thumb/2098814416359653377/img/8W7uRq67dQVcC7we?format=webp&name=medium";
const X_OTHER = "https://pbs.twimg.com/amplify_video_thumb/2096974405414821888/img/ck4Gh9VO6ELGiAvt?format=webp&name=medium";

describe("posterIds", () => {
  it("finds the id a poster shares with its media", () => {
    expect(posterIds(X_POST)).toEqual(["2098814416359653377"]);
    expect(posterIds("https://pbs.twimg.com/ext_tw_video_thumb/1846000000000000001/pu/img/AbC9.jpg")).toEqual([
      "1846000000000000001",
    ]);
  });

  it("leaves sizes, random names and missing posters out", () => {
    expect(posterIds("https://cdn.example.net/thumbs/1280x720/a8B3cD9eF2gH.jpg")).toEqual([]);
    expect(posterIds("https://cdn.example.net/poster.jpg")).toEqual([]);
    expect(posterIds("blob:https://x.com/1")).toEqual([]);
    expect(posterIds(undefined)).toEqual([]);
  });
});

describe("mediaForVideo", () => {
  const list = [
    { url: "https://video.twimg.com/amplify_video/2096974405414821888/pl/6CgnpnCvzLgr5FZr.m3u8?tag=29&v=447" },
    { url: "https://video.twimg.com/amplify_video/2098814416359653377/pl/4n8R-J-RweGcpX1z.m3u8?tag=29&v=996" },
  ];

  it("offers the hovered video's stream on a page with several", () => {
    expect(mediaForVideo(list, X_POST).map((c) => c.url)).toEqual([list[1]?.url]);
    expect(mediaForVideo(list, X_OTHER).map((c) => c.url)).toEqual([list[0]?.url]);
  });

  it("falls back to the whole list", () => {
    expect(mediaForVideo(list, undefined)).toEqual(list);
    expect(mediaForVideo(list, "https://pbs.twimg.com/amplify_video_thumb/1111111111111111111/img/x.jpg")).toEqual(list);
  });
});
