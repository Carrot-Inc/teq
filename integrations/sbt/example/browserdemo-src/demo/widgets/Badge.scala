package demo.widgets

import scala.scalajs.js

import demo.facade.React

/** A second file of the per-file package, which `Widgets` imports: an edit here runs this
  * module and `Widgets`'s again. */
object Badge:
  val text: String = "badge: plain"

  val View: js.Function0[js.Any] = () =>
    React.createElement("span", js.Dynamic.literal(id = "badge"), text)
