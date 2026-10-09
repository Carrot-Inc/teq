//> using options -language:strictEquality
// teq: --strict-equality
// An abstract type compares with `null` under strict equality through a given `CanEqual`.
def isNull[A](a: A): Boolean =
  given CanEqual[A, Null] = CanEqual.derived
  a == null

def notNull[A](a: A)(using CanEqual[Null, A]): Boolean = null != a

@main def run(): Unit =
  println(isNull("x"))
  println(isNull[String](null))
  given CanEqual[Null, Int] = CanEqual.derived
  println(notNull(3))
