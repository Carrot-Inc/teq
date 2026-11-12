/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "japgolly.scalajs.react.vdom.HtmlAttrs.cls",
  "japgolly.scalajs.react.vdom.HtmlAttrs.className",
  "japgolly.scalajs.react.vdom.HtmlAttrs.classSet",
  "japgolly.scalajs.react.vdom.HtmlAttrs.classSet1",
  "japgolly.scalajs.react.vdom.HtmlAttrs.classSet1M",
  "japgolly.scalajs.react.vdom.HtmlAttrs.classSetM",
  "japgolly.scalajs.react.vdom.ImplicitsForVdomNode.TraversableOnceExt.toVdomArray"
]
*/
package fix

import japgolly.scalajs.react.*
import japgolly.scalajs.react.vdom.html_<^.*

object ReactAttributes:
  val cls = <.div(^.cls := "x") // assert: ForbiddenSymbol
  val className = <.div(^.className := "y") // assert: ForbiddenSymbol
  val classSet = <.div(^.classSet("a" -> true)) // assert: ForbiddenSymbol
  val classSet1 = <.div(^.classSet1("a", "b" -> true)) // assert: ForbiddenSymbol
  val classSetM = <.div(^.classSetM(Map("a" -> true))) // assert: ForbiddenSymbol
  val classSet1M = <.div(^.classSet1M("a", Map("b" -> true))) // assert: ForbiddenSymbol
  val array = <.ul(List(1, 2).map(i => <.li(i)).toVdomArray) // assert: ForbiddenSymbol
  val arrayOf = List(1, 2).toVdomArray(i => <.li(i)) // assert: ForbiddenSymbol
  val allowed = <.div(^.id := "x")
  val anchored = <.div(^.cls := "x") // scalafix:ok ForbiddenSymbol
  val anchoredWithin = <.div(
    ^.cls := "x" // scalafix:ok ForbiddenSymbol
  )
  val anchoredBare = <.div(^.cls := "x") // scalafix:ok
  val anchoredUnused = <.div(^.id := "unused") /* scalafix:ok ForbiddenSymbol */ // assert: UnusedScalafixSuppression

object Renamed:
  import ^.{cls as klass} // scalafix:ok ForbiddenSymbol
  val use = <.div(klass := "z") // assert: ForbiddenSymbol
  val anchored = <.div(klass := "z") // scalafix:ok ForbiddenSymbol

object RenamedBare:
  import ^.{className as name} // assert: ForbiddenSymbol
  val use = <.div(name := "z") // assert: ForbiddenSymbol
