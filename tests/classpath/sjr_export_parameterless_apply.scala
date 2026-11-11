// jars: scala-library scalajs-react
// Parameterless exports apply their jar-loaded result. TagLite's inherited
// opaque Tag keeps the identity used by the apply extension in its implicit scope.
//> using scala 3.8.4
//> using platform scala-js
//> using dep com.github.japgolly.scalajs-react::core-bundle-cats_effect::4.0.0
package exportedtags
import japgolly.scalajs.react.vdom.TagMod
import japgolly.scalajs.react.vdom.all.vdomNodeFromString
trait Tags:
  export japgolly.scalajs.react.vdom.HtmlTags.{main as mainTag, *}
object Page extends Tags:
  def empty = div()
  def text = div("hello")
  def mods(xs: TagMod*) = div(xs*)
