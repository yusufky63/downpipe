// Builds dist/chrome (Chrome, Edge) and dist/firefox from the same sources.
// Each script is bundled into one classic file: Chrome's service worker and
// Firefox's event page then load it the same way, without module support.
import { createRequire } from "node:module";
import { copyFile, cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "esbuild";

const root = fileURLToPath(new URL("..", import.meta.url));
const require = createRequire(import.meta.url);
const pkg = JSON.parse(await readFile(join(root, "package.json"), "utf8"));

const TARGETS = {
  chrome: "chrome120",
  firefox: "firefox128",
};

const FONTS = [
  ["@fontsource/dm-sans", "dm-sans-latin-400-normal.woff2"],
  ["@fontsource/dm-sans", "dm-sans-latin-500-normal.woff2"],
  ["@fontsource/space-grotesk", "space-grotesk-latin-500-normal.woff2"],
  ["@fontsource/jetbrains-mono", "jetbrains-mono-latin-400-normal.woff2"],
];

for (const [target, esTarget] of Object.entries(TARGETS)) {
  const out = join(root, "dist", target);
  await rm(out, { recursive: true, force: true });
  await mkdir(join(out, "fonts"), { recursive: true });

  await build({
    absWorkingDir: root,
    entryPoints: { background: "src/background.ts", popup: "src/popup/popup.ts" },
    outdir: out,
    bundle: true,
    format: "iife",
    target: esTarget,
    legalComments: "none",
    logLevel: "warning",
  });

  await copyFile(join(root, "src/popup/popup.html"), join(out, "popup.html"));
  await copyFile(join(root, "src/popup/popup.css"), join(out, "popup.css"));
  await cp(join(root, "public/icons"), join(out, "icons"), { recursive: true });
  for (const [pkgName, file] of FONTS) {
    const dir = dirname(require.resolve(`${pkgName}/package.json`));
    await copyFile(join(dir, "files", file), join(out, "fonts", file));
  }

  const source = JSON.parse(await readFile(join(root, "manifests", `${target}.json`), "utf8"));
  const { manifest_version, name, ...rest } = source;
  const manifest = { manifest_version, name, version: pkg.version, ...rest };
  await writeFile(join(out, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  console.log(`built dist/${target}`);
}
