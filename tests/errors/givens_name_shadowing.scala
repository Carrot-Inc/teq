// expect: 13:30: error: no given instance of type Show[Int] was found for parameter x
// expect: 1 error found
// A given of a nearer scope that fits the wanted type hides the givens of its name further
// out, even when its own using clause cannot be filled: scalac's shadowing by implicit name.
trait Show[T] { def n: String }
trait Foo[T]
given x: Show[Int] with { def n = "outer x" }
given y: Show[String] with { def n = "outer y" }
object O:
  given x[A](using Foo[A]): Show[A] with { def n = "inner x" }
  // y does not fit Show[Int], so it hides nothing.
  given y: Show[String] with { def n = "inner y" }
  def run = summon[Show[Int]].n
@main def main(): Unit =
  println(O.run)
