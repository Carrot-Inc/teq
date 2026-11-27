// Imports the module compiled from js_val_before_init_freeze.scala; run by tests/run_interop.sh.
import { create } from "../../out/interop/js_val_before_init_freeze.mjs";
const A = create().constructor;
console.log(Object.getOwnPropertyNames(A.prototype).includes("f"));
Object.freeze(A.prototype);
try { console.log(create().f); } catch (e) { console.log(e.name); }
