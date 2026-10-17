import { describe, expect, it } from "vitest";
import {
  DEFAULT_MEDIA_MIN_BYTES,
  DEFAULT_MEDIA_TYPES,
  MAX_MEDIA_TYPES,
  MAX_TAB_MEDIA,
  addCandidate,
  classifyResponse,
  classifySource,
  cleanTitle,
  defaultMediaFilters,
  excludeMatchesFor,
  frameOf,
  isPartOf,
  kindLabel,
  mediaExtension,
  mediaName,
  pageHeaders,
  parseMediaFilters,
  parseMediaTypeList,
  parseTabMedia,
  responseSize,
  tabMediaKey,
  titleFileName,
  validMediaFilters,
  withoutRangeParams,
  type MediaCandidate,
  type ResponseLike,
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
      mediaTypes: [...DEFAULT_MEDIA_TYPES],
      mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES,
    });
    expect(parseMediaFilters({ mediaOverlay: false, mediaTypes: ["MP4", ".mkv", "bad type", 3], mediaMinBytes: -1 })).toEqual({
      mediaOverlay: false,
      mediaTypes: ["mp4", "mkv"],
      mediaMinBytes: DEFAULT_MEDIA_MIN_BYTES,
    });
  });

  it("accept only stored forms from the options page", () => {
    expect(validMediaFilters({ mediaOverlay: true, mediaTypes: ["mp4"], mediaMinBytes: 0 })).toEqual({
      mediaOverlay: true,
      mediaTypes: ["mp4"],
      mediaMinBytes: 0,
    });
    for (const bad of [
      { mediaOverlay: true, mediaTypes: ["MP4"], mediaMinBytes: 0 },
      { mediaOverlay: true, mediaTypes: ["mp4", "mp4"], mediaMinBytes: 0 },
      { mediaOverlay: "yes", mediaTypes: [], mediaMinBytes: 0 },
      { mediaOverlay: true, mediaTypes: [], mediaMinBytes: 1.5 },
      { mediaOverlay: true, mediaTypes: [], mediaMinBytes: 0, extra: 1 },
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

describe("addCandidate", () => {
  it("adds newest first and ignores repeats and fragments", () => {
    let list: MediaCandidate[] = [];
    list = addCandidate(list, { ...base, url: "https://v.example.net/a/master.m3u8#x", referrer: undefined }, "a", 1) ?? list;
    expect(list).toEqual([{ ...base, url: "https://v.example.net/a/master.m3u8", id: "a", at: 1 }]);
    expect(addCandidate(list, { ...base, url: "https://v.example.net/a/master.m3u8" }, "b", 2)).toBeNull();
    // The quality and audio playlists the master lists.
    expect(addCandidate(list, { ...base, url: "https://v.example.net/a/720p/index.m3u8" }, "c", 3)).toBeNull();
    const next = addCandidate(list, { kind: "direct", url: "https://v.example.net/b.mp4", size: 9, origin: "https://p.example" }, "d", 4);
    expect(next?.map((c) => c.id)).toEqual(["d", "a"]);
    expect(next?.[0]?.origin).toBe("https://p.example");
  });

  it("keeps playlists from other folders and sites apart", () => {
    const list = [{ ...base, url: "https://v.example.net/a/master.m3u8", id: "a", at: 1 }];
    expect(addCandidate(list, { ...base, url: "https://v.example.net/a/other.m3u8" }, "b", 2)).toHaveLength(2);
    expect(addCandidate(list, { ...base, url: "https://w.example.net/a/hd/index.m3u8" }, "c", 2)).toHaveLength(2);
  });

  it("keeps a bounded list", () => {
    let list: MediaCandidate[] = [];
    for (let i = 0; i < MAX_TAB_MEDIA + 5; i++) {
      list = addCandidate(list, { kind: "direct", url: `https://v.example.net/${i}.mp4` }, `id${i}`, i) ?? list;
    }
    expect(list).toHaveLength(MAX_TAB_MEDIA);
    expect(list[0]?.id).toBe(`id${MAX_TAB_MEDIA + 4}`);
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
        null,
      ]),
    ).toEqual([good]);
    expect(parseTabMedia("nope")).toEqual([]);
    expect(tabMediaKey(12)).toBe("media:12");
  });
});

describe("labels and names", () => {
  it("names the kind of media", () => {
    expect(kindLabel({ kind: "hls", url: "https://a.org/x.m3u8" })).toBe("HLS stream");
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

  it("cleans page titles", () => {
    expect(cleanTitle("  A\tvideo\n title  ")).toBe("A video title");
    expect(cleanTitle("\u0000\u0007")).toBeUndefined();
    expect(cleanTitle(undefined)).toBeUndefined();
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
