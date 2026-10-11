// The parents dotty's `Desugar.classDef` gives a case class and a case object, `Product` and
// `Serializable`, and an enum, `scala.reflect.Enum`, which extends both: they are types of the
// values and tests of them pass, a written `Product` taken once.
case class C(x: Int)
case class W(x: Int) extends Product
case object O
enum E:
  case A
  case B(y: Int)

@main def run(): Unit =
  val a: java.io.Serializable = C(1)
  val b: java.io.Serializable = O
  val c: java.io.Serializable = E.A
  val d: Serializable = E.B(2)
  val p: Product = W(1)
  val e: scala.reflect.Enum = E.B(3)
  println(p.productArity + " " + e.ordinal)
  println(List[Any](a, b, c, d, W(2)).map(_.isInstanceOf[java.io.Serializable]))
  println(List[Any](C(1), O, E.A, E.B(1), W(2)).map(_.isInstanceOf[Product]))
  println(List[Any](C(1), E.A).map(_.isInstanceOf[scala.reflect.Enum]))
