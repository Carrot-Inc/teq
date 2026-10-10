// A refinement's members are completed on demand, so an earlier member reads a later one's info
// (a projection, an application), and the refinement's own `this` is its self
// (`Typer.typedRefinedTypeTree`'s class scope, `TypeAssigner.assignType`).
import scala.annotation.nowarn

@nowarn type R = AnyRef { type A = B#X; type B = AnyRef { type X = Int } }
@nowarn type H = AnyRef { type A = B[Int]; type B[X] = List[X] }

trait P { type T; def f: Any }
type T1 = P { def f: this.T }
class K extends P { type T = String; def f: T = "this" }
def use(r: T1): r.T = r.f

@main def run(): Unit =
  val r: R = new AnyRef { type B = AnyRef { type X = Int }; type A = Int }
  val a: r.A = 3
  val h: H = new AnyRef { type B[X] = List[X]; type A = List[Int] }
  val l: h.A = List(4)
  println(a + l.head)
  println(use(new K))
