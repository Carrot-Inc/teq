// A function type's `=>` inside a guard's type arguments, then a blank line and a leading `&&`.
def test[A]: Boolean = true
@main def run(): Unit =
  println(1 match
    case n if test[Int => Int]

    && true => n
    case _ => 0)
