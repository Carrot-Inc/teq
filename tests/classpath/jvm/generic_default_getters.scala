// jars: scala-library
// std: scala-library
// Defaults whose parameter's type names the method's type parameters: teq's getters are typed as the parameter in
// the pickle and the class file, where scalac's have the default's type (`a$default$1()String`), since teq's typer
// infers the method's type arguments without the getter's; what the calls print is scalac's.
object O:
  def a[A](x: A = "hi"): A = x
  def b[A <: CharSequence](x: A = "hi"): A = x
  def c[A](x: List[A] = Nil): Int = x.size
  def d[A](x: A)(y: List[A] = List(x, x)): Int = y.size
  def e[A](x: Option[A] = None): Int = x.size
  def f[A](x: A = 1): A = x
class K[T]:
  def m(x: T = null.asInstanceOf[T]): T = x
  def n[A](x: A = 3.5): A = x
object Main:
  def main(args: Array[String]): Unit =
    println(O.a())
    println(O.b())
    println(O.c())
    println(O.d(5)())
    println(O.e())
    println(O.f())
    println(new K[String].m())
    println(new K[String].n())
