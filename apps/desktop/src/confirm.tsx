import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "./styles.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import { ConfirmWindow } from "./ConfirmWindow";
import { inTauri } from "./lib/api";
import { progressId } from "./lib/progress";
import { ProgressWindow } from "./ProgressWindow";

// The confirmation window and every download's progress window load this
// page; the window's label says which it is.
const download = inTauri() ? progressId(getCurrentWebviewWindow().label) : null;

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(<StrictMode>{download !== null ? <ProgressWindow id={download} /> : <ConfirmWindow />}</StrictMode>);
}
