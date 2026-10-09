# vite-plugin-scalajs-teq (retired)

This was a copy of the Scala.js vite plugin with two options naming the sbt tasks to print for the
output directory, so that it could serve teq's output through `teqLinkJS`/`teqFullLinkJS`. It is
retired: with `TEQ_COMPILER=1` in sbt's environment, sbt-teq answers a Scala.js project's own
`fastLinkJS` and `fullLinkJS` from teq's output (`../sbt/README.md`, "teq as the compiler"), with
the report and the output-directory attribute the stock plugin's `fastLinkJSOutput` and
`fullLinkJSOutput` read, so `@scala-js/vite-plugin-scalajs` serves teq's output unmodified:

```js
// vite.config.js
import scalajs from "@scala-js/vite-plugin-scalajs"

export default defineConfig({
  plugins: [scalajs({ projectID: "frontend" })],
})
```

`sbt ~frontend/fastLinkJS` keeps the directory current in dev; `vite build` runs `fullLinkJS`
itself through the plugin. `../sbt/example/` drives its browser page this way
(`vite.scalajs.config.js`, `check-scalajs-dev.mjs`).
