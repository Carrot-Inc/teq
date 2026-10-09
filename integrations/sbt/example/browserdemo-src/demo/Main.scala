package demo

import scala.scalajs.js

import demo.facade.{React, ReactDOMClient, Root}
import demo.widgets.Widgets

/** `main.mjs` runs this again on every hot swap, since it is in the chain a dev server
  * re-executes (docs/TARGETS.md, "The vite plugin"): the root is kept on `window` across
  * re-runs, so a swap re-renders the existing tree instead of mounting a second one. */
@main def run(): Unit =
  val page = js.Dynamic.global.window
  page.__demoStamp = "one"
  val root =
    if js.isUndefined(page.__demoRoot) then
      val created = ReactDOMClient.createRoot(js.Dynamic.global.document.getElementById("root"))
      page.__demoRoot = created
      created
    else page.__demoRoot.asInstanceOf[Root]
  root.render(React.createElement(Widgets.App, null))
