package demo.widgets

import scala.scalajs.js

import demo.Labels
import demo.facade.React
import demo.model.{Chips, Theme, Themes}

/** An enum of this file, of which `SizeSwitch` holds a value in hook state once its button
  * was clicked: what a swap of this file does to that value is what the browser check pins
  * (docs/TARGETS.md, "Module splitting"). */
enum Size:
  case Small, Large

/** The page a hot swap edits: `greeting` is the line the dev-loop measurement changes.
  * `--module-per-file demo.widgets` makes this file its own module, so an edit re-executes it
  * alone (docs/TARGETS.md, "Module splitting"); the hook state of `Counter` and `ThemeSwitch`
  * survives the swap because React Fast Refresh (`runtime/hot-refresh.mjs`) tells React they
  * are the same component families under a new implementation, and the `Theme` that
  * `ThemeSwitch` holds still matches because `demo.model`'s module is not run again, while
  * its `Themes.label` reads the `themePrefix` of the module the swap made. */
object Widgets:
  val greeting: String = "Hello from teq"
  val themePrefix: String = "theme: "

  private def button(id: String, onClick: js.Function1[js.Any, Unit], text: String): js.Any =
    React.createElement("button", js.Dynamic.literal(id = id, onClick = onClick), text)

  val Counter: js.Function0[js.Any] = () =>
    val state = React.useState(0)
    val count = state(0).asInstanceOf[Int]
    val set = state(1).asInstanceOf[js.Function1[js.Any, Unit]]
    button("counter", _ => set(count + 1), s"count: $count")

  val ThemeSwitch: js.Function0[js.Any] = () =>
    val state = React.useState(Theme.Light.asInstanceOf[js.Any])
    val theme = state(0).asInstanceOf[Theme]
    val set = state(1).asInstanceOf[js.Function1[js.Any, Unit]]
    button("theme", _ => set(Themes.next(theme).asInstanceOf[js.Any]), Themes.label(theme))

  val SizeSwitch: js.Function0[js.Any] = () =>
    val state = React.useState(js.undefined)
    val set = state(1).asInstanceOf[js.Function1[js.Any, Unit]]
    val label =
      if js.isUndefined(state(0)) then "size: none"
      else
        state(0).asInstanceOf[Size] match
          case Size.Small => "size: small"
          case Size.Large => "size: large"
    button("size", _ => set(Size.Large.asInstanceOf[js.Any]), label)

  val App: js.Function0[js.Any] = () =>
    React.createElement(
      "div",
      null,
      React.createElement("h1", null, greeting),
      React.createElement(Counter, null),
      React.createElement(ThemeSwitch, null),
      React.createElement(SizeSwitch, null),
      React.createElement(Badge.View, null),
      if Chips.isChip(Chip.first) then React.createElement(Chip.View, null) else null,
      React.createElement("p", js.Dynamic.literal(id = "footer"), Labels.footer),
    )
