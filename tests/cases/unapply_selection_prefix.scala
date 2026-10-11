// A case class's synthetic `unapply` selected on a companion named through a prefix is the
// companion's member seen from the qualifier (dotty's `trySelectUnapply` selects `unapply` on it):
// it takes the class through that prefix (`Use.this.Product.Extraction`, `b.In`), beside an
// overloaded `unapply` of the companion's own, in a cake, in the class and through a value.
trait Cake:
  object Product:
    case class Extraction[A](a: A)
    object Extraction:
      def unapply[A](x: Int): Option[Int] = Some(x)
trait Use extends Cake:
  def f(e: Use.this.Product.Extraction[String]): String = e match
    case Product.Extraction(a) => a
object U extends Use
object Shapes:
  case class Circle(r: Int)
  object Circle:
    def unapply(x: Int): Option[Int] = Some(x)
class Box(val n: Int):
  case class In(v: Int)
  object In:
    def unapply(s: String): Option[Int] = Some(s.length)
  def f(i: In): Int = i match
    case In(v) => v + n
def g(c: Shapes.Circle): Int = c match
  case Shapes.Circle(r) => r
def h(b: Box, i: b.In): Int = i match
  case b.In(v) => v

@main def run(): Unit =
  println(U.f(U.Product.Extraction("ok")))
  val b = new Box(10)
  println(g(Shapes.Circle(3)))
  println(b.f(b.In(4)))
  println(h(b, b.In(5)))
