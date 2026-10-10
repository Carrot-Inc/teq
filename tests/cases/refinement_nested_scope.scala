// Refinements inside refinements: an inner member names the outer refinement's members past its
// own binder, which is kept only where its own members name it (`RecType.closeOver`).
type N = AnyRef { type A; type B = AnyRef { type C = A; type D } }
type N2 = AnyRef { type A; type B <: AnyRef { type C; type D = C; type E = A } }
class I { type C = Int; type D = Int; type E = String }
class O { type A = String; type B = I }
def f(n: N)(x: n.A): n.A = x
def g(n: N2)(b: n.B)(c: b.C): b.D = c
def h(n: N2)(b: n.B)(e: b.E): n.A = e
@main def run(): Unit =
  val o = O()
  val n: N2 { type A = String; type B = I } = o
  val i = I()
  val b: n.B = i
  println(g(n)(b)(1))
  println(h(n)(b)("ok"))
  val m: N { type A = String } = new AnyRef { type A = String; type B = AnyRef { type C = String; type D } }
  println(f(m)("f"))
