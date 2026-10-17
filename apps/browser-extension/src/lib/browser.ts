// Firefox puts promise-returning APIs on `browser`; Chrome's Manifest V3
// `chrome.*` APIs return promises too, so one code path serves both.
const firefox = (globalThis as { browser?: typeof chrome }).browser;

export const ext: typeof chrome = firefox ?? chrome;
