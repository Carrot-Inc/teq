// A default's expected type is the parameter's with the method's own parameters approximated
// (`Namer.expectedDefaultArgType`, `ProtoTypes.wildApprox` over the whole type): a type
// parameter under a refinement's alias (`Box { type T = A }`) is a wildcard there too, so a
// `new Box { type T = Int }` default conforms; a dependent default (`y: x.type = x`) keeps its
// path in the getter's type (an application of it is rejected by scalac too: the getter's result
// is `Int` against the argument's singleton).
trait Box { type T }
object Test {
  def f[A](x: Box { type T = A } = new Box { type T = Int }): Int = 1
  def g(x: Int)(y: x.type = x): x.type = y
}
object Main:
  def main(args: Array[String]): Unit =
    println(Test.f())
    println(Test.g(5)(5))
