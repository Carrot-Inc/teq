trait A { def n: Int }
trait X { def result: Int }
class C(val a: A)
class B(val a: A)

object A { given A with { def n = 0 } }
object C { given c(using a: A): C = new C(a) }
object B { given b(using c: C): B = new B(c.a) }

type XA = (X & A) { type T = Int }

object X {
  given root(using b: => B): XA = new X with A {
    type T = Int
    def n = 1
    def result = b.a.n
  }
}

val warm = summon[C]
@main def main(): Unit = println(summon[XA].result)
