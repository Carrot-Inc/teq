// jars: scala-library scalajs-react
// targets: js
// scalajs-react 4.0.0 from its jars: `Reusability.derive` (a macro) for case classes, a sealed
// trait, an enum, `Reusability.by`, `caseClassExcept`, the decimal and map instances, `Reusable.fn`. The macros expand
// from the jars (`tests/classpath/js/sjr_reusability_derive.scala` is this probe without its last line); stops at the
// library body `Reusability.optionLike` (`o.fold(x, o.isEmpty(y))(..)`: "value apply of Boolean").
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
package sjrreuse

import japgolly.scalajs.react.*
import japgolly.scalajs.react.Reusability.DecimalImplicitsWithoutTolerance.reusabilityDouble
import japgolly.scalajs.react.Reusability.MapImplicits.reusabilityMap

final case class Point(x: Int, y: Int)
final case class Shape(name: String, points: List[Point], scale: Double, tags: Map[String, Int])

sealed trait Status
object Status:
  case object Idle extends Status
  final case class Busy(job: String) extends Status
  final case class Failed(code: Int, retry: Boolean) extends Status

enum Color:
  case Red, Green
  case Custom(hex: String)

given Reusability[Point] = Reusability.derive
given Reusability[Shape] = Reusability.derive
given Reusability[Status] = Reusability.derive
given Reusability[Color] = Reusability.derive

final case class Tagged(id: Long, label: String)
given Reusability[Tagged] = Reusability.by(_.id)

final case class Wide(a: Int, b: String, c: Boolean)
given Reusability[Wide] = Reusability.caseClassExcept("b")

def show[A](label: String, a: A, b: A)(using r: Reusability[A]): Unit =
  println(s"$label: ${r.test(a, b)}")

@main def main(): Unit =
  show("point same", Point(1, 2), Point(1, 2))
  show("point diff", Point(1, 2), Point(1, 3))
  val s = Shape("s", List(Point(0, 0)), 1.5, Map("a" -> 1))
  show("shape same", s, s.copy())
  show("shape scale", s, s.copy(scale = 2.0))
  show("shape tags", s, s.copy(tags = Map("a" -> 2)))
  show[Status]("idle", Status.Idle, Status.Idle)
  show[Status]("busy", Status.Busy("a"), Status.Busy("a"))
  show[Status]("busy diff", Status.Busy("a"), Status.Busy("b"))
  show[Status]("cross", Status.Idle, Status.Failed(1, true))
  show[Color]("color", Color.Red, Color.Red)
  show[Color]("custom", Color.Custom("#fff"), Color.Custom("#000"))
  show("tagged", Tagged(1, "a"), Tagged(1, "b"))
  show("wide", Wide(1, "x", true), Wide(1, "y", true))
  show("wide c", Wide(1, "x", true), Wide(1, "x", false))
  val r = Reusable.fn((i: Int) => i + 1)
  println(r(41))
  show("option", Option(Point(1, 1)), Option(Point(1, 1)))
