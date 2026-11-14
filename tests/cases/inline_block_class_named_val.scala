// An inline method whose body declares a class with a val beside an inherited method of its name: the val's
// accessor goes by a name of its own in the output, a state of the member's record, which each expansion's copy
// of the class carries with the record (before, a build stopped with `a named method has its name`).
trait Gauge:
  def level(step: Int): Int = step

object Dials:
  inline def reading(k: Int): Int =
    class Dial extends Gauge:
      val level: Int = k
    val d = new Dial
    d.level + d.level(2)

@main def run(): Unit =
  println(Dials.reading(1))
  println(Dials.reading(10))
