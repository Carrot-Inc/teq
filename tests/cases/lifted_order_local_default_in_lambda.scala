// A local def inside a lambda, called with its default: the default getter is asked while the lambda's body is lowered.
@main def main(): Unit = {
  val f = () => { def g(a: Int = 1)(b: Int) = a + b; g()(2) }
  val h = () => 3
  println(f() + h())
}
