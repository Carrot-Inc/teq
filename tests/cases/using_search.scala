// `summon` in a class's constructor code finds the class's own using parameter accessor, an `Ident` as an inferred
// argument (`tpd.ref`), which the constructor reads as its parameter (`Constructors.intoConstr`), in a closure there
// too; the field is the subclass's override.
class A()(using val s: Int):
  val direct = s
  val saved = summon[Int]
  val closure = () => summon[Int]
class B()(using override val s: Int) extends A()(using s + 1)

@main def run(): Unit =
  val b = new B()(using 2)
  println(b.s)
  println(b.direct)
  println(b.saved)
  println(b.closure())
