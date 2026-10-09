// What a page pays for the split output before the program runs: the time node takes to load
// (parse, compile, evaluate) every module but main.mjs, then main.mjs on top, which also runs
// $init, $enums and the entry point. Usage: node bench/load.mjs <split dir> [runs]
import { readdirSync, statSync } from "node:fs";
import { pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";

const dir = process.argv[2];
const runs = Number(process.argv[3] ?? 5);
const files = readdirSync(dir).filter((f) => f.endsWith(".mjs")).sort();
const bytes = files.reduce((n, f) => n + statSync(`${dir}/${f}`).size, 0);

if (process.argv[4] === "--once") {
  const t0 = performance.now();
  for (const f of files) if (f !== "main.mjs") await import(pathToFileURL(`${dir}/${f}`).href);
  const t1 = performance.now();
  await import(pathToFileURL(`${dir}/main.mjs`).href);
  const t2 = performance.now();
  console.error(JSON.stringify({ modules: t1 - t0, main: t2 - t1 }));
  process.exit(0);
}

const samples = [];
for (let i = 0; i < runs; i++) {
  const r = spawnSync(process.execPath, [process.argv[1], dir, "1", "--once"], { encoding: "utf8" });
  const line = r.stderr.trim().split("\n").pop();
  samples.push(JSON.parse(line));
}
const median = (xs) => xs.slice().sort((a, b) => a - b)[Math.floor(xs.length / 2)];
console.log(
  `${files.length} modules, ${bytes} bytes: load all but main ${median(samples.map((s) => s.modules)).toFixed(1)} ms, ` +
    `main.mjs (init, enums, entry point) ${median(samples.map((s) => s.main)).toFixed(1)} ms (median of ${runs})`
);
