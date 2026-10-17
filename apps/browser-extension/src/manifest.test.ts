import { describe, expect, it } from "vitest";
import rustManifest from "../../../crates/native-host/src/manifest.rs?raw";
import chrome from "../manifests/chrome.json";
import firefox from "../manifests/firefox.json";
import { HOST_NAME } from "./lib/protocol";
import linksHtml from "./links/links.html?raw";
import optionsHtml from "./options/options.html?raw";
import popupHtml from "./popup/popup.html?raw";

/** Chrome's extension id: SHA-256 of the public key, first 16 bytes, hex digits mapped to a-p. */
async function extensionId(base64Key: string): Promise<string> {
  const der = Uint8Array.from(atob(base64Key), (c) => c.charCodeAt(0));
  const hash = new Uint8Array(await crypto.subtle.digest("SHA-256", der));
  return [...hash.slice(0, 16)]
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("")
    .replace(/[0-9a-f]/g, (h) => String.fromCharCode(97 + parseInt(h, 16)));
}

function rustConst(name: string): string | undefined {
  return new RegExp(`pub const ${name}: &str = "([^"]+)";`).exec(rustManifest)?.[1];
}

describe("manifests", () => {
  it("ask for host access only as an option, for media detection", () => {
    for (const m of [chrome, firefox] as Record<string, unknown>[]) {
      expect(m.host_permissions).toBeUndefined();
      // Requested when the user turns media detection on, removed when off.
      expect(m.optional_host_permissions).toEqual(["http://*/*", "https://*/*"]);
      // The overlay button is registered at run time, and only while detection is on.
      expect(m.content_scripts).toBeUndefined();
      expect(m.externally_connectable).toBeUndefined();
      // The links page is opened by the extension itself; web pages cannot load it.
      expect(m.web_accessible_resources).toBeUndefined();
      expect(JSON.stringify(m)).not.toContain("<all_urls>");
    }
  });

  it("keep permissions minimal and downloads optional", () => {
    for (const m of [chrome, firefox]) {
      // activeTab + scripting: reading a page's links once, after a click.
      expect([...m.permissions].sort()).toEqual([
        "activeTab",
        "contextMenus",
        "nativeMessaging",
        "notifications",
        "scripting",
        "storage",
      ]);
      expect(m.optional_permissions).toEqual(["downloads", "webRequest"]);
    }
  });

  it("open the capture filters as the options page, in a tab", () => {
    for (const m of [chrome, firefox]) {
      expect(m.options_ui).toEqual({ page: "options.html", open_in_tab: true });
      expect(m.action.default_popup).toBe("popup.html");
    }
  });

  it("have pages that load the scripts and styles the build writes", () => {
    const pages: [string, string, boolean][] = [
      ["popup", popupHtml, false],
      ["options", optionsHtml, true],
      ["links", linksHtml, true],
    ];
    for (const [name, html, fullPage] of pages) {
      expect(html, name).toContain(`<script src="${name}.js" defer></script>`);
      expect(html, name).toContain('<link rel="stylesheet" href="base.css" />');
      expect(html, name).toContain(`<link rel="stylesheet" href="${name}.css" />`);
      expect(html.includes('href="page.css"'), name).toBe(fullPage);
      // No inline scripts: Manifest V3's content security policy would block them.
      expect(html.match(/<script/g)?.length, name).toBe(1);
    }
  });

  it("pin the Chrome id the host manifest allows", async () => {
    expect(await extensionId(chrome.key)).toBe(rustConst("CHROMIUM_EXTENSION_ID"));
  });

  it("use the Firefox id and host name the host knows", () => {
    expect(firefox.browser_specific_settings.gecko.id).toBe(rustConst("FIREFOX_EXTENSION_ID"));
    expect(rustConst("HOST_NAME")).toBe(HOST_NAME);
  });

  it("load the background script the way each browser needs", () => {
    expect(chrome.background).toEqual({ service_worker: "background.js" });
    expect(firefox.background).toEqual({ scripts: ["background.js"] });
  });
});
