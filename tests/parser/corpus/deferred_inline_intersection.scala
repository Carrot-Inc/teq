// An overloaded deferred inline member's implementation is found in every class of the
// argument's own type: an intersection in either order, a refinement, a singleton's underlying.
trait A:
  def apply(x: Int): String
  def apply(x: String): String
trait B extends A:
  override inline def apply(x: Int): String
  def apply(x: String): String = "string"
trait D
class C extends B:
  inline def apply(x: Int): String = "int" + x
class E extends C with D:
  def extra: Int = 7
inline def f(x: B): String = x(1) + " " + x("s")
@main def run(): Unit =
  val x: D & C = new E
  println(f(x))
  val y: C & D = new E
  println(f(y))
  val z: C { def extra: Int } = new E
  println(f(z))
  val e = new E
  println(f(e))
