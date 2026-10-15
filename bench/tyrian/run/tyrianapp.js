// The sandbox's page script (sandbox/tyrianapp.js) over teq's build of sandbox.js, served beside it as
// out/: the same launch, of the module teq writes in place of Mill's fastLinkJS.dest/main.js.
import { TyrianApp } from "./out/main.mjs";

TyrianApp.launch(document.getElementById("myapp"), {
  message: "Hello, Tyrian!",
});
