import { encode } from "uqr";

import { qrPath } from "../lib/remote";

/**
 * A QR code drawn as one SVG path. Dark modules on white in both themes:
 * phone cameras read inverted codes poorly, and the quiet zone around it
 * has to be light too.
 */
export function QrCode({ text, label, size = 184 }: { text: string; label: string; size?: number }) {
  const qr = encode(text, { ecc: "M", border: 0 });
  const quiet = 4;
  const box = qr.size + quiet * 2;
  return (
    <svg
      role="img"
      aria-label={label}
      width={size}
      height={size}
      viewBox={`${-quiet} ${-quiet} ${box} ${box}`}
      shapeRendering="crispEdges"
      className="shrink-0 rounded-md border border-line"
    >
      <rect x={-quiet} y={-quiet} width={box} height={box} fill="#ffffff" />
      <path d={qrPath(qr.data)} fill="#0a0b0d" />
    </svg>
  );
}
