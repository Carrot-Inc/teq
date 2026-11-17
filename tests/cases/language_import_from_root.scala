// teq: --feature --werror
// A language import written from the root (`_root_.scala.language.X`) enables its feature as `scala.language.X`
// and `language.X` do (dotty's `TreeInfo.languageImport`): no feature warning for the conversion below.
import _root_.scala.language.implicitConversions

case class Meters(n: Int)
implicit def toMeters(n: Int): Meters = Meters(n)

@main def run(): Unit =
  val m: Meters = 3
  println(m)
