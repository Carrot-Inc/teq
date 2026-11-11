// An operator selected on a receiver whose type is a parameter with a bound: the bound's member, as a selection
// finds it (`String.+` for an `S <: String`, `Int.+` for an `N <: Int`), and the inline bodies are written.
object Lib:
  inline def append[S <: String](s: S): Any => String = s.+
  inline def more[N <: Int](n: N): Int = n + 1

@main def run(): Unit =
  println(Lib.append[String]("x")(1))
  println(Lib.more[Int](41))
