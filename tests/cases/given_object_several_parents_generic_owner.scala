// A given object of several parents in a generic class is found as each of its parents, the class's type parameter
// seen from the instance or the object it is read on (dotty's `c.pair.type`, whose base types are seen from the
// prefix): `P[T]` of `pair` is a `P[Int]` read on a `C[Int]`, `S[T]` of `both` an `S[Int]` read on a `D[Int]`.
// scalac 3.8.4 prints `p5`, `p6`, `s5` and `s61`.
class P[T] { def p(t: T): String = "p" + t.toString }
trait R { def r: Int = 2 }
class C[T]:
  given pair: P[T]() with R with {}
object O extends C[Int]
class Q { def x: Int = 1 }
trait S[T] { def s(t: T): String = "s" + t.toString }
class D[T]:
  given both: Q() with S[T] with {}
object E extends D[Int]
@main def run(): Unit =
  val c = C[Int]()
  locally:
    import c.given
    println(summon[P[Int]].p(5))
  locally:
    import O.given
    println(summon[P[Int]].p(6))
  val d = D[Int]()
  locally:
    import d.given
    println(summon[S[Int]].s(5))
  locally:
    import E.given
    println(summon[S[Int]].s(6) + summon[Q & S[Int]].x)
