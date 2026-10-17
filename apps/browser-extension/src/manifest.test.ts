import { describe, expect, it } from "vitest";
import rustManifest from "../../../crates/native-host/src/manifest.rs?raw";
import chrome from "../manifests/chrome.json";
import firefox from "../manifests/firefox.json";
import { HOST_NAME } from "./lib/protocol";

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
  it("ask for no host access at all", () => {
    for (const m of [chrome, firefox] as Record<string, unknown>[]) {
      expect(m.host_permissions).toBeUndefined();
      expect(m.content_scripts).toBeUndefined();
      expect(m.externally_connectable).toBeUndefined();
      expect(JSON.stringify(m)).not.toContain("<all_urls>");
    }
  });

  it("keep permissions minimal and downloads optional", () => {
    for (const m of [chrome, firefox]) {
      expect([...m.permissions].sort()).toEqual(["contextMenus", "nativeMessaging", "notifications", "storage"]);
      expect(m.optional_permissions).toEqual(["downloads"]);
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
