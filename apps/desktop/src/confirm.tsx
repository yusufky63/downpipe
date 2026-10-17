import "@fontsource/dm-sans/400.css";
import "@fontsource/dm-sans/500.css";
import "@fontsource/space-grotesk/500.css";
import "@fontsource/space-grotesk/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "./styles.css";

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { ConfirmWindow } from "./ConfirmWindow";

const root = document.getElementById("root");
if (root) {
  createRoot(root).render(
    <StrictMode>
      <ConfirmWindow />
    </StrictMode>,
  );
}
