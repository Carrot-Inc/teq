// An enum nested in a class is typed through its enclosing instance: its value cases, a class case's
// creation and the companion's lookups through a path are of the enum through that path (`a.E`).
class O:
  enum E:
    case A, B
    case K(n: Int)
  enum V:
    case X, Y
  def pick: E = E.A
  def all: List[V] = V.values.toList
val a = new O
val b = new O
val y: b.E = b.E.A
val z: a.E = a.pick
val k: a.E = a.E.K(1)
val l: List[a.V] = a.all
def f(o: O)(e: o.E): Int = e.ordinal
@main def run(): Unit =
  println(f(a)(a.E.B))
  println(y)
  println(z == a.E.A)
  println(k)
  println(l)
  println(a.V.valueOf("Y") == a.V.Y)
  val m: a.E = a.E.fromOrdinal(1)
  println(m)
  val w: a.V = a.V.fromOrdinal(1)
  println(w)
  println((b.E.A: b.E) match { case b.E.A => "A"; case _ => "other" })
