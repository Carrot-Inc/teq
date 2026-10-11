// A structural given is a module (dotty's `Parsers.givenDef`), whose information is its class: met
// with another part's member of its name on an intersection (`Denotation.meet`), the joint member
// has the class's members and the other's (`ab.g.extra`, a `P & Q` besides).
trait P { def x: Int = 1 }
trait Q { def y: String = "q" }
trait A { given g: P with { def extra = 2 } }
trait B { val g: Q }
def f(ab: A & B): (Int, Int) = (ab.g.x, ab.g.extra)
def h(ab: A & B): P & Q = ab.g

@main def run(): Unit =
  val a = new A {}
  println(a.g.extra + a.g.x)
  val typed: (A & B) => (Int, Int) = f
  println(typed != null)
