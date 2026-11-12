// The callee a case class whose companion is written without an apply.
case class Kept(a: Int)
object Kept:
  val zero: Kept = Kept(0)
def on[A, B](f: A => B): A => B = f
@main def run(): Unit =
  val f = on(Kept(_))
  println(f(1))
