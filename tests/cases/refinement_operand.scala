// A refinement without a parent as the operand of `&` (dotty's `Parsers.simpleType1`: a
// `RefinedTypeTree` of no parent, which refines `Object`): `A & { type T = Int }` fixes `A`'s `T`
// for a path through it; any brace starts one, `A & {}` and a `{}` after a `:` too.
trait A:
  type T
  def get: T
def f(a: A & { type T = Int })(x: a.T): Int = x + 1
type Keyed = A & { type T = String }
def g(k: Keyed): String = k.get + "!"
trait B
val empty: B & {} = new B {}
val bare: {} = new B {}
def h(b: B & {} & A): Boolean = b.isInstanceOf[B]

@main def run() =
  val a = new A { type T = Int; def get = 3 }
  println(f(a)(41))
  println(g(new A { type T = String; def get = "s" }))
  println((empty.isInstanceOf[B], bare.isInstanceOf[B], h(new B with A { type T = Int; def get = 1 })))
