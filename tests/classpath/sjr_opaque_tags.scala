// jars: scala-library scalajs-react
// scalajs-react 4.0.0's tags from its jars: `HtmlTagOf.Tag[N]`, an opaque type of the trait `TagLite` reached
// through the object `HtmlTagOf`, spelt as a path (`div`) and through the exported alias `HtmlTagOf[N]` (`br`);
// the extension `name` of the trait is in the implicit scope through the object prefix.
// Checked only: running it waits for the library bodies the tags reach (`new js.Array()`); under scala-cli --js with
// the directives below it prints div, br, span and circle.
//> using scala 3.8.4
//> using platform scala-js
//> using dep com.github.japgolly.scalajs-react::core-bundle-cats_effect::4.0.0
package sjropaque

import japgolly.scalajs.react.vdom.html_<^.*
import japgolly.scalajs.react.vdom.{HtmlTagOf, SvgTagOf}

@main def main(): Unit =
  println(<.div.name)
  println(<.br.name)
  val t: HtmlTagOf.Tag[org.scalajs.dom.html.Span] = <.span
  println(t.name)
  println(SvgTagOf[org.scalajs.dom.svg.Circle]("circle").name)

// Through `vdom.all`'s val `HtmlTagOf`, which names the object: the path's type member is the object's copy.
object ThroughVal:
  import japgolly.scalajs.react.vdom.all.{HtmlTagOf as AllTagOf, id}
  val dialog: AllTagOf.Tag[org.scalajs.dom.HTMLDialogElement] = AllTagOf[org.scalajs.dom.HTMLDialogElement]("dialog")
  val withId = dialog(id := "d")
