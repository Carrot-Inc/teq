// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: `ScalaComponent.builder` with a backend, initial state from
// props, `getDerivedStateFromProps`, lifecycle hooks, an error boundary over `render_PCS` and a static component.
// Checks clean since the fixes for the vdom's tags, the class path's first copy of a class, the naming
// macros expanded from the jar. A build stops in bodies at the std's Scala.js layer (`js.Object.create` and
// `defineProperty`, `js.constructorOf`, `js.ThisFunction.fromFunction1`, which `ViaReactComponent` builds React's
// class with) and in the retained body of `ScalaJsReactConfig.Instance.modifyComponentName`, an inline override whose
// runtime version teq types from the inline body (scalac pickles it expanded, as `modifyComponentName$retainedBody`).
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
package sjrclass

import japgolly.scalajs.react.*
import japgolly.scalajs.react.callback.*
import japgolly.scalajs.react.vdom.html_<^.*
import scala.scalajs.js

final case class Props(title: String, items: List[String])
final case class State(count: Int, note: String)

final class Backend($: BackendScope[Props, State]):
  def bump = $.modState(s => s.copy(count = s.count + 1))
  def render(p: Props, s: State): VdomElement =
    <.div(
      <.h1(p.title),
      <.p(s"count ${s.count} ${s.note}"),
      <.ul(p.items.map(i => <.li(^.key := i, i)).toVdomArray),
      <.button(^.onClick --> bump, "+"),
    )

val Counter = ScalaComponent.builder[Props]("Counter")
  .initialStateFromProps(p => State(p.items.length, "init"))
  .backend(new Backend(_))
  .renderPS(($, p, s) => $.backend.render(p, s))
  .getDerivedStateFromProps((p, s) => s.copy(note = s"derived from ${p.title}"))
  .componentDidMount(_ => Callback.log("mounted"))
  .shouldComponentUpdatePure(_ => true)
  .componentWillUnmount(_ => Callback.empty)
  .build

final case class FallbackProps(error: js.Any)

val Boundary = ScalaComponent.builder[FallbackProps => VdomNode]("Boundary")
  .initialState[Option[js.Any]](None)
  .noBackend
  .render_PCS((render, children, caught) =>
    caught match
      case Some(error) => render(FallbackProps(error))
      case None        => children
  )
  .getDerivedStateFromError(error => Some(error))
  .build

val Static = ScalaComponent.static(<.hr())

@main def main(): Unit =
  println(ReactDOMServer.renderToStaticMarkup(Counter(Props("Items", List("a", "b", "c")))))
  println(ReactDOMServer.renderToStaticMarkup(Boundary(f => <.b(s"failed"))(<.i("child one"), <.i("child two"))))
  println(ReactDOMServer.renderToStaticMarkup(Static()))
  println(Counter.displayName)
