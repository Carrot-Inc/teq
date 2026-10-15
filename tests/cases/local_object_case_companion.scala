// A case class of an object of a block has the companion `Desugar.classDef` gives it, an object of
// that object's instance, its `Mirror.Product` and a value; its class files have the companion
// read its enclosing instance from the capture the block's object passes it, there being no outer
// accessor to call (the JVM's written members), as its `apply`, `fromProduct` and defaults do;
// its `fromProduct` passes the block's values the class captures, as a local enum's class case's
// companion's does.
import scala.deriving.Mirror

@main def run(): Unit =
  def local(n: Int): String =
    object O:
      case class C(x: Int, tag: String = "t" + n)
    val lm = summon[Mirror.ProductOf[O.C]]
    (lm eq summon[Mirror.ProductOf[O.C]], lm.fromProduct((7, "p")), O.C, O.C(3), O.C(4, "q")).toString
  println(local(1))
  println(local(2))
  def cases(n: Int): String =
    enum Loc:
      case P(x: Int, y: Int = n)
      case Q
    val lm = summon[Mirror.ProductOf[Loc.P]]
    (lm eq summon[Mirror.ProductOf[Loc.P]], lm.fromProduct((7, 8)), Loc.P(1), Loc.P).toString
  println(cases(3))
