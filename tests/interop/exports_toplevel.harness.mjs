// Imports the module compiled from exports_toplevel/; run by tests/run_interop.sh.
import { osModule, twice, version } from "../../out/interop/exports_toplevel.mjs";

console.log("harness start");
console.log(`${version} ${typeof osModule.platform} ${twice(21)}`);
