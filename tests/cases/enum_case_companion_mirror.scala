// An enum's class case has the companion `Desugar.classDef` gives a case class, an object of the
// enum's companion, and that companion is its `Mirror.Product` (`useCompanionAsProductMirror`):
// two summons are the same object, as scalac's are, for an enum of a package, of a class and of a
// block; the companion is a value (`Draft.Email`, its `toString` the case's name), its `apply`
// constructs and widens to the enum, an expected function takes it eta-expanded, and a pattern
// reads the case through it.
import scala.deriving.Mirror

enum Draft:
  case Store(title: Option[String])
  case Email(subject: Option[String], on: Boolean)
  case Empty

class Outer:
  enum In:
    case A(x: Int)
    case B

@main def run(): Unit =
  case class L(a: Int)
  val m1 = summon[Mirror.ProductOf[Draft.Email]]
  val m2 = summon[Mirror.ProductOf[Draft.Email]]
  println(m1.fromProduct((None, true)))
  println(m1 eq m2)
  val l1 = summon[Mirror.ProductOf[L]]
  val l2 = summon[Mirror.ProductOf[L]]
  println((l1 eq l2, l1.fromProduct(Tuple1(3))))
  val e = Draft.Email(Some("s"), false)
  val ev = summon[e.type <:< Draft]
  println(e)
  val xs = List(Draft.Store(None), Draft.Email(None, true))
  val ys: List[Draft] = xs
  println(ys)
  val f = Draft.Email
  println(f)
  val g: (Option[String], Boolean) => Draft = Draft.Email.apply
  println(g(None, false))
  Draft.Email(Some("t"), true) match
    case Draft.Email(Some(s), on) => println(s"$s $on")
    case _ => println("other")
  val o = new Outer
  val a1 = summon[Mirror.ProductOf[o.In.A]]
  println((a1 eq summon[Mirror.ProductOf[o.In.A]], a1.fromProduct(Tuple1(4))))
  println(o.In.A(5))
  def local(): String =
    enum Loc:
      case P(x: Int)
      case Q
    val lm = summon[Mirror.ProductOf[Loc.P]]
    (lm eq summon[Mirror.ProductOf[Loc.P]], lm.fromProduct(Tuple1(7)), Loc.P).toString
  println(local())
