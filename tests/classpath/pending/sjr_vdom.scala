// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: the vdom DSL, `<.div(^.className := .., ^.onClick --> ..)`,
// `==>` with a keyboard event, `TagMod`, `TagMod.when`, `VdomNode`, `EmptyVdom`, `VdomArray`, `ReactFragment`, a
// style from `js.Dictionary`, `toTagMod`. Since the typer's fixes for the tags, `<.br()`, `js.Dictionary` as a
// `js.Any` and by-name parameters of function types, it checks clean; the build stops in bodies at the std's Scala.js
// layer: `js.Object`'s companion ops on a `js.Any` (`JsUtil.objectIterator`), `new js.Symbol`, `js.Array.push`'s
// `Int`.
// The expectation is scalac 3.8.4's: `scala-cli --power package <file> -o out.mjs` with the directives below (the
// application's five artifacts, in its order), run by node from a directory whose parent chain has `react` and
// `react-dom` 19 in a node_modules.
//> using scala 3.8.4
//> using platform scala-js
//> using jsModuleKind es
//> using dep com.github.japgolly.scalajs-react::callback::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats::4.0.0
//> using dep com.github.japgolly.scalajs-react::callback-ext-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::core-bundle-cats_effect::4.0.0
//> using dep com.github.japgolly.scalajs-react::extra::4.0.0
package sjrvdom

import japgolly.scalajs.react.*
import japgolly.scalajs.react.callback.*
import japgolly.scalajs.react.vdom.html_<^.*
import japgolly.scalajs.react.feature.ReactFragment
import org.scalajs.dom
import scala.scalajs.js

def row(label: String, value: Option[String]): VdomNode =
  <.tr(<.td(label), value.fold[VdomNode](EmptyVdom)(v => <.td(^.className := "value", v)))

val mods: TagMod = TagMod(^.id := "main", ^.title := "t", ^.style := js.Dictionary("color" -> "red"))


def keyHandler(e: ReactKeyboardEventFromHtml): Callback =
  Callback.log(e.key)

def onKey(e: facade.SyntheticKeyboardEvent[dom.HTMLInputElement]): Callback = Callback.empty

@main def main(): Unit =
  val flag = true
  val node: VdomNode = <.div(
    ^.className := "box",
    mods,
    ^.onClick --> Callback.log("click"),
    ^.onKeyDown ==> keyHandler,
    ^.disabled := false,
    TagMod.when(flag)(^.cls := "on"),
    TagMod.unless(flag)(^.cls := "off"),
    <.span("x").when(flag),
    <.input.text(^.value := "v", ^.readOnly := true, ^.onKeyUp ==> onKey),
    List(1, 2).toTagMod(using i => <.em(i)),
    EmptyVdom,
    "text",
    42,
    <.table(<.tbody(row("a", Some("1")), row("b", None))),
    VdomArray(<.i(^.key := 1, "one"), <.i(^.key := 2, "two")),
    ReactFragment(<.u("f1"), <.u("f2")),
  )
  println(ReactDOMServer.renderToStaticMarkup(node))
  println(ReactDOMServer.renderToStaticMarkup(<.br()))
  val attrs = <.a(^.href := "#", ^.target.blank, "link")
  println(ReactDOMServer.renderToStaticMarkup(attrs))
