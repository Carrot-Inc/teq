// A class case whose constructor has defaults, of an enum of a block (made inner to its
// companion, which takes what the block's values the case reads) and of a class, and a class
// nested in a class: each default stands by its own parameter after the outer instance the class
// takes first (`ExplicitOuter`), seeing the block's values and the parameters before it, as
// `Desugar.classDef` gives the constructor its default getters.
def local(n: Int): String =
  enum E:
    case A
    case B(x: Int = n, y: Int = n * 2)
    case C(s: String = "c")(t: Int = s.length + n)
  val b1: E.B = E.B()
  val b2: E.B = E.B(1)
  val b3: E.B = E.B(y = 3)
  val c1: E.C = E.C()()
  val c2: E.C = E.C("long")()
  s"${b1.x} ${b1.y} ${b2.x} ${b2.y} ${b3.x} ${b3.y} ${c1.s} ${c2.s} $c1 $c2"

class Holder(val k: Int):
  enum Kind:
    case K(v: Int = k + 1)

class Outer(val k: Int):
  class Inner(val x: Int = k + 1, val y: Int = 2)

@main def run(): Unit =
  println(local(7))
  val h = new Holder(10)
  val k1: h.Kind.K = h.Kind.K()
  val k2: h.Kind.K = h.Kind.K(2)
  println(k1.v)
  println(k2.v)
  val o = new Outer(20)
  val i = new o.Inner()
  println(s"${i.x} ${i.y} ${new o.Inner(y = 5).x}")
