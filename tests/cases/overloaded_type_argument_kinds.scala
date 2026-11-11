// Overloaded methods applied to a written type argument: the alternatives whose parameter's bounds the
// argument meets stay (dotty's resolveOverloaded, TypeOps.boundsViolations). A type lambda is within no
// first-order parameter's bounds; a higher-kinded parameter without a bound takes any constructor.
def choose[A](s: String): Int = 1
def choose[F[_]](s: Any): Int = 2

// A bound that only the written arguments make `Any` (`B <: A` at `A := Any`) is the instantiated one.
def bounded[A, B <: A](x: String): Int = 1
def bounded[A, B](x: Any): Int = 2

class Box[F[_]]:
  def this(n: Int) = this()
  def kind: String = "constructor"

@main def run(): Unit =
  println(choose[[X] =>> Int](""))
  println(choose[Int](""))
  println(new Box[Option](1).kind)
  println(bounded[Any, Int](""))
