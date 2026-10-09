//> using scala 3.8.4
object o:
  private[o] trait A
  trait B { def name = "b" }
  class C extends A, B
  class D extends A, B

trait Named { def name: String }
trait HasSize { def size: Int }
case class NS1() extends Named, HasSize { def name = "ns1"; def size = 1 }
case class NS2() extends Named, HasSize { def name = "ns2"; def size = 2 }

def pick[T](x: T): T => T = identity

@main def run(): Unit =
  val c = true
  val g = pick(if c then o.C() else o.D())
  println(g(new o.B {}).name)
  val h: Named & HasSize = if c then NS1() else NS2()
  println(h.name + h.size)
