// The implementation of a deferred given is a lazy given of the implementing class: the search's
// value is computed on the first read and kept, whatever reads it (`Typer.implementDeferredGivens`).
import scala.compiletime.deferred

trait T:
  given x: Int = deferred
  def viaSummon = summon[Int]

object Definition:
  var n: Int = 0
  given fresh(using DummyImplicit): Int = { n += 1; println("made " + n); n }
  class C extends T

@main def run(): Unit =
  val c = new Definition.C
  println("constructed " + Definition.n)
  println(c.x)
  println(c.viaSummon)
  println(c.x)
  println(Definition.n)
  val d = new Definition.C
  println(s"${d.x} ${Definition.n}")
