// A case class whose companion is written with an apply of the same count: the application has two
// alternatives and no callee is read from it (scalac 3.8.4: E134, none of the overloaded alternatives of
// method apply match the argument).
// expect: 10:14: error: None of the overloaded alternatives of method apply in object Kept with types
case class Kept(a: Int)
object Kept:
  def apply(s: String): Kept = Kept(s.length)
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Kept(_))
  println(f(1))
