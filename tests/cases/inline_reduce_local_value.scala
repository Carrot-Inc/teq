// A reducible inline match whose pattern is a local value (an `Equals` pattern on a local
// object): the object's own plain match is copied per expansion, as with an extractor.
inline def f[T](x: T): Int =
  object C:
    def value(y: T): Int = y match
      case _: Int => 1
      case _ => 2
  inline x match
    case C => 0
    case _ => C.value(x)

@main def run(): Unit =
  println(f[Int](1))
  println(f[String]("a"))
