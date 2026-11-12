// An inline `unapply` whose body declares a local lazy val or a local object is expanded inside the pattern that
// calls it: the lazy val gets its holder, accessor and initialiser wherever the pattern stands, in a `match`, a
// pattern definition, a `catch`, a `for` and a nested pattern, from this file and from another of the build
// (dotty's LazyVals.transformLocalDef meets the expansion in the pattern's tree).
import lib.Twice

object Thrice:
  inline def unapply(x: Int): Option[Int] = { lazy val tripled: Int = x * 3; Some(tripled) }

class Boom(val n: Int) extends Exception
object Code:
  inline def unapply(e: Boom): Option[Int] = { lazy val c: Int = e.n + 1; Some(c) }

object WithObject:
  inline def unapply(x: Int): Option[Int] = { object O { val v = x + 5 }; Some(O.v) }

@main def run(): Unit =
  println(21 match { case Twice(d) => d; case _ => -1 })
  println(-1 match { case Twice(d) => d; case _ => -1 })
  println(7 match { case Thrice(d) if d > 20 => d; case _ => -1 })
  val Thrice(t) = 4: @unchecked
  println(t)
  println(try throw new Boom(1) catch { case Code(c) => c })
  println(for case Twice(x) <- List(1, -2, 3) yield x)
  println(Option(5) match { case Some(Thrice(Twice(n))) => n; case _ => -1 })
  println(1 match { case WithObject(d) => d; case _ => -1 })
