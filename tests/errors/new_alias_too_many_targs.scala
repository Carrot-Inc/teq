// `new` of a parameterised alias takes the alias's type arguments, not its class's.
class C[A, B]()
type T[X] = C[String, X]
@main def run(): Unit =
  val bad = new T[Boolean, Int]()
  println(bad)
// expect: Too many type arguments for T
