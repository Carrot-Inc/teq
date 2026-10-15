// A mirror summoned in its companion's own body is the companion under construction (`tpd.ref`
// gives a module as `This` of its class where the code stands in it), not its accessor, which the
// companion of a class nested in a class would call again while it is made.
import scala.deriving.Mirror
class Outer(val p: String):
  case class R(x: Int)
  object R:
    val m = summon[Mirror.ProductOf[R]]
    def make = m.fromProduct(Tuple1(7))
  def run = R.make
case class T(x: Int)
object T:
  val m = summon[Mirror.ProductOf[T]]
case object O:
  val m = summon[Mirror.ProductOf[O.type]]
@main def run(): Unit =
  val o = Outer("p")
  println(o.run)
  println(o.R.m eq o.R)
  println(T.m eq T)
  println(O.m eq O)
