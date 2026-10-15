// The companion `Desugar.classDef` makes for a case class nested in a class is an object of each
// instance, the case's `apply` and extractor, printing the class's name; a trait's alike.
class Outer:
  case class C(x: Int)
  def make(i: Int): C = C(i)
trait Holder:
  case class D(s: String)
object H extends Holder
@main def run(): Unit =
  val o = new Outer
  println(o.C)
  println(o.make(1))
  println(o.C(2) match { case o.C(x) => x })
  println(o.C eq o.C)
  println(H.D("d"))
  println(H.D)
  val o2 = new WithDefault
  println(o2.E)
  println(o2.E())
  println(o2.mirror.fromProduct(Tuple1(5)))
class WithDefault:
  case class E(x: Int = 1)
  def mirror = summon[scala.deriving.Mirror.ProductOf[E]]
