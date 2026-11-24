// A named tuple's values are covariant (NamedTuple[N, +V]), its names invariant: a named tuple widens to one with the
// same names and wider values wherever it is compared, the file's first definitions included, where the class's
// type parameters are met before anything else of the program reads them.
object Widen:
  val u: (a: Int, b: Int) = (a = 1, b = 2)
  val t: (a: Any, b: Int) = u
  def arg(x: (a: Any, b: Int)): Any = x.a
  def res(x: (a: Int, b: String)): (a: AnyVal, b: CharSequence) = x
  val fn: ((a: Any, b: Int)) => Int = x => x.b
  val narrowed: ((a: Int, b: Int)) => Int = fn
  val xs: List[(a: Matchable, b: Int)] = List(u)
object Infer:
  // a variable met only contravariantly, through a named tuple's function, is maximized
  def mk[T >: Int](): (a: T => Unit, b: Int) = (a = (t: T) => println(t), b = 0)
  val r = mk()
  val q: (a: Any => Unit, b: Int) = r
@main def run(): Unit =
  println(Widen.t.a)
  println(Widen.arg(Widen.u))
  println(Widen.res((a = 3, b = "s")).a)
  println(Widen.narrowed((a = 4, b = 5)))
  println(Widen.xs.head.a)
  Infer.q.a("ok")
