// The match of an inline method's body is checked where it is defined, once, however many calls expand it and
// with none at all: scalac 3.8.4 warns at each definition and at no call.
inline def partial(x: Option[Int]): Int = x match { case Some(v) => v }
inline def unused(x: Option[Int]): Int = x match { case Some(v) => v }
@main def run(): Unit =
  val g = partial(Some(1)) + partial(Some(2))
  println(g)
