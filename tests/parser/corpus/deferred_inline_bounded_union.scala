// A deferred inline member's implementation is found through a type parameter's upper bound
// and a union's common class.
trait A:
  def apply(x: Int): String
  def apply(x: String): String
trait B extends A:
  override inline def apply(x: Int): String
  def apply(x: String): String = "string"
class C extends B:
  inline def apply(x: Int): String = "ok" + x
class D extends C
class E extends C
inline def f(x: B): String = x(1) + " " + x("s")
def bounded[T <: C](x: T): String = f(x)
@main def run(): Unit =
  println(bounded(new C))
  println(bounded(new D))
  val x: D | E = new D
  println(f(x))
  val y: D | E = new E
  println(f(y))
