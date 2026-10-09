// A PNG of the repository's logo centred on the page's background, for what cannot take an SVG: the card a link
// shows when shared, and the icon a phone keeps for a bookmark.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { Resvg } from "@resvg/resvg-js";

const background = "#fbfaf8";

export function logoPng(file: string, width: number, height: number, logoWidth: number): Response {
  const logo = readFileSync(join(process.cwd(), "..", "assets", "logo", file), "utf-8");
  const [, , vw, vh] = logo.match(/viewBox="([^"]+)"/)![1].split(/\s+/).map(Number);
  const logoHeight = (logoWidth * vh) / vw;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}">
<rect width="100%" height="100%" fill="${background}"/>
<image x="${(width - logoWidth) / 2}" y="${(height - logoHeight) / 2}" width="${logoWidth}" height="${logoHeight}"
  href="data:image/svg+xml;base64,${Buffer.from(logo).toString("base64")}"/>
</svg>`;
  const png = new Resvg(svg).render().asPng();
  return new Response(new Uint8Array(png), { headers: { "Content-Type": "image/png" } });
}
