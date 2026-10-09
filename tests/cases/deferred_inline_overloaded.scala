// An overloaded deferred inline member: the alternative is resolved on the parameter's
// declared type, its implementation found in the argument's class.
trait A:
  def apply(x: Int): String
  def apply(x: String): String
trait B extends A:
  override inline def apply(x: Int): String
  def apply(x: String): String = "string"
class C extends B:
  inline def apply(x: Int): String = "int" + x
object O extends B:
  inline def apply(x: Int): String = "object" + x
inline def f(x: B): String = x(1) + " " + x("s")
@main def run(): Unit =
  println(f(new C))
  println(f(O))
