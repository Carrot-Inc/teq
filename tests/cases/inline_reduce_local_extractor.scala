// A reducible inline match whose pattern calls a local extractor: the object's own plain match
// is copied per expansion and never shares its pattern with the record, so the second call with
// another type argument is not settled by the first.
inline def f[T](x: T): Int =
  object C:
    def unapply(y: Any): Boolean = false
    def value(y: T): Int = y match
      case _: Int => 1
      case _ => 2
  inline x match
    case C() => 0
    case _ => C.value(x)

@main def run(): Unit =
  println(f[Int](1))
  println(f[String]("a"))
