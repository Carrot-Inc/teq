trait A { def n: Int }
class B(val a: A)
class C(val a: A)
trait Root extends A { def result: Int }

object A { given A with { def n = 0 } }
object C { given c(using a: A): C = new C(a) }
object B { given b(using c: C): B = new B(c.a) }
object Root {
  given root(using b: => B): Root with {
    def n = 1
    def result = b.a.n
  }
}
object Test {
  val warm = summon[C]
  val result = summon[Root].result
}
@main def main(): Unit = println(Test.result)
