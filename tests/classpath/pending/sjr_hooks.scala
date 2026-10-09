// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: function components with props and hooks, the library's
// `useState` in a for over `HookResult` and the raw facade's `React.useState` as the application calls it, rendered
// with `ReactDOMServer.renderToStaticMarkup`. Checks clean since the typer's fix of the tags' opaque type
// seen through the object prefix `HtmlTagOf.Tag[N]`; the build stops in the body of `Hooks.apply`, which
// core-generic's jar typed against util-fallbacks' `DefaultEffects` (`[A] =>> Function0[A]`) and teq re-types
// against the bundle's (`SyncIO`).
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
package sjrhooks

import japgolly.scalajs.react.*
import japgolly.scalajs.react.vdom.html_<^.*
import japgolly.scalajs.react.facade.React as RawReact
import scala.scalajs.js

final case class Props(name: String, start: Int)

val Counter = ScalaFnComponent[Props] { p =>
  for
    count <- useState(p.start)
    label <- useState("clicks")
  yield <.div(^.className := "counter", s"${p.name}: ${count.value} ${label.value}")
}

val RawHooks = ScalaFnComponent[Int] { n =>
  val pair = RawReact.useState[Int]((() => n * 2): js.Function0[Int])
  <.span(s"raw ${pair._1}")
}

@main def main(): Unit =
  println(ReactDOMServer.renderToStaticMarkup(Counter(Props("a", 1))))
  println(ReactDOMServer.renderToStaticMarkup(<.section(Counter(Props("b", 7)), RawHooks(21))))
