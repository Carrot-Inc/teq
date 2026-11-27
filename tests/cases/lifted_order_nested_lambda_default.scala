// A local def with a default inside a lambda inside a lambda: the getter, asked in a later round, takes its place
// before both lambdas, which hold it.
@main def main(): Unit = {
  val f = (x: Int) => { val g = (y: Int) => { def d(z: Int = y) = z + x; d() }; g(x) }
  println(f(3))
}
