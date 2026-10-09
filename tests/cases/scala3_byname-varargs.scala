// Adapted from scala3 tests/run/byname-varargs.scala (Apache-2.0, see tests/scala3/README.md).
def f[T](xs: => T*): (Seq[T], Seq[T]) = (xs, xs)

def a =
  println("a")
  1

def b =
  println("b")
  2

@main def Test =
  println(s"result = ${f(a, b)}")