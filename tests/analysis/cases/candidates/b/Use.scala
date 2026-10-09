package cnb

import scala.language.implicitConversions
import cna.*
import cna.Ops.grow
import cna.Meters.given

// Alternatives the typer tries and drops: the overloads not chosen, the given candidates of
// another type, the extension that needs the conversion first.
object Use:
  def show[A](a: A)(using s: Show[A]): String = s.show(a)
  val a: String = Ops.pick(1)
  val b: String = Ops.pick("x")
  val c: String = show(3)
  val d: Box = 5
  val e: Int = new Box(2).grow.n
  val f: String = Ops.pick(new Box(3))
