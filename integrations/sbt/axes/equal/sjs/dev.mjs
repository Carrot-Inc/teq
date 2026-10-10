// sjs's dev command (package.json's dev script, which the export names): records its arguments and the binary teq
// dev names, then waits to be ended, as a dev server does.
import { writeFileSync } from "node:fs";

writeFileSync("dev-started", `${process.argv.slice(2).join(" ")} ${process.env.TEQ ? "with TEQ" : "without TEQ"}\n`);
setInterval(() => {}, 1000);
