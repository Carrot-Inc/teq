// Parses every ES module under the given directories without running it and prints the early
// errors, a name declared twice among them; exits 1 if there is one. Needs --experimental-vm-modules.
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { SourceTextModule } from "node:vm";

let found = 0;
const walk = (p) => {
  if (!existsSync(p)) return;
  if (statSync(p).isDirectory()) for (const e of readdirSync(p)) walk(join(p, e));
  else if (p.endsWith(".mjs")) {
    try {
      new SourceTextModule(readFileSync(p, "utf8"), { identifier: p });
    } catch (e) {
      console.log(`${p}: ${e.message}`);
      found++;
    }
  }
};
for (const p of process.argv.slice(2)) walk(p);
process.exit(found ? 1 : 0);
