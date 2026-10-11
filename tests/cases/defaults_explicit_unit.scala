// An argument written `()` is a value like any other: only an argument the source leaves out
// applies the parameter's default, a parameter of type `Unit`, `Any` or a type parameter
// included, whatever value the target gives `()`.
object ExplicitUnit:
  def f(a: Unit = print("D")): Unit = ()
  def g(a: Any = { print("G"); 1 }): Any = a
  def h[T](a: T = { print("H"); null.asInstanceOf[T] }): T = a
  class K(val a: Unit = print("K"))

  def main(args: Array[String]): Unit =
    f(()); println("end")
    f(); println("end")
    println(g(()) == ())
    println(g())
    println(h[Unit](()) == ())
    val k1 = new K(()); println(k1.a == ())
    val k2 = new K(); println(k2.a == ())
