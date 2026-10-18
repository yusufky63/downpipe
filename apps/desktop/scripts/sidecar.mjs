// Builds downpipe-native-host and puts it where `bundle.externalBin` in
// tauri.conf.json expects it: src-tauri/binaries/downpipe-native-host-<target triple>.
// The Tauri CLI runs this before `tauri dev` and `tauri build` and says which
// target and profile it is building through TAURI_ENV_* variables; run by
// hand, it builds a release host for this machine.
import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const PACKAGE = "downpipe-native-host";
const tauriDir = fileURLToPath(new URL("../src-tauri/", import.meta.url));

const hostTriple = /^host: (\S+)$/m.exec(execFileSync("rustc", ["-vV"], { encoding: "utf8" }))?.[1];
if (!hostTriple) throw new Error("rustc -vV did not report a host triple");
const triple = process.env.TAURI_ENV_TARGET_TRIPLE || hostTriple;
const debug = process.env.TAURI_ENV_DEBUG === "true";

// Builds the host for one target and returns the executable Cargo wrote.
function build(target) {
  const args = ["build", "-p", PACKAGE, "--bin", PACKAGE, "--message-format=json-render-diagnostics"];
  if (!debug) args.push("--release");
  // Without --target the host lands in target/<profile>/, next to a development
  // build of the app, which finds it there (see manifest::host_beside).
  if (target !== hostTriple) args.push("--target", target);

  console.log(`sidecar: cargo ${args.join(" ")}`);
  const cargo = spawnSync("cargo", args, {
    cwd: tauriDir,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  });
  if (cargo.error) throw cargo.error;
  if (cargo.status !== 0) process.exit(cargo.status ?? 1);

  // Cargo reports where it wrote the executable, whatever the target folder is.
  const executable = cargo.stdout
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line))
    .find((msg) => msg.reason === "compiler-artifact" && msg.target?.name === PACKAGE && msg.executable)?.executable;
  if (!executable) throw new Error(`cargo did not report the ${PACKAGE} executable`);
  return executable;
}

const extension = triple.includes("windows") ? ".exe" : "";
const dest = join(tauriDir, "binaries", `${PACKAGE}-${triple}${extension}`);
mkdirSync(join(tauriDir, "binaries"), { recursive: true });
if (triple === "universal-apple-darwin") {
  // `tauri build --target universal-apple-darwin` wants one sidecar that runs
  // on both Apple Silicon and Intel Macs; Cargo has no such target, so build
  // each and join them.
  const parts = ["aarch64-apple-darwin", "x86_64-apple-darwin"].map(build);
  execFileSync("lipo", ["-create", "-output", dest, ...parts], { stdio: "inherit" });
} else {
  copyFileSync(build(triple), dest);
}
console.log(`sidecar: ${dest}`);
