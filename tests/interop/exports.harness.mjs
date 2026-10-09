// Imports the module compiled from exports.scala; run by tests/run_interop.sh.
import api, * as shop from "../../out/interop/exports.mjs";
import {
  version, catalog, settings, greet, total, label, product, describe, cheaperThan, applyTwice, log,
  fromJs, basename,
} from "../../out/interop/exports.mjs";

console.log("harness start");
console.log(Object.keys(shop).sort().join(" "));
console.log(version, JSON.stringify(settings));
console.log(greet("node"));
console.log(`${total()} ${total(5)} ${total(1, 2, 3)}`);
console.log(label("p"), label("p", "a", "b"));

const pencil = product("pencil");
console.log(`${describe(pencil)} / ${pencil.name} / ${pencil.price} / ${pencil}`);
console.log(describe(product("desk", 250)));

// teq collections are iterable, and their methods are plain JS methods
console.log([...catalog].map(describe).join("; "));
console.log(Array.from(cheaperThan(10), (p) => p.name).join(","));
for (const p of cheaperThan(100)) console.log("for-of", p.name);
console.log(`${catalog.length()} ${catalog.map((p) => p.price)}`);

// parameter clauses are flattened: applyTwice(f)(x) in teq
console.log(String(applyTwice((x) => x * 3, 2)));
console.log(String(fromJs(["a", "b"])));
console.log(basename("/x/y.txt"));
console.log(api.greet("default export"), api.version);

// output of the module and of the harness stays in order
log("first");
console.log("between");
log("second");
