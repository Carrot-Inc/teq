// A closure in a default has no getter of the method's: `Desugar.addDefaultGetters` gives the
// method alone getters, and the closure's parameter takes what its call passes, `()` included.
def mark(s: String): Int = { println(s); 1 }
class K:
  def run(n: Int = mark("n"), cb: Unit => Int = u => { println(s"cb $u"); 1 }): Int = cb(()) + n
  def pair(n: Int = mark("m"), f: (Unit, Int) => Int = (u, k) => { println(s"f $u $k"); k }): Int = f((), 7) + n
class Sub extends K:
  override def run(n: Int, cb: Unit => Int): Int = cb(()) - n
class C(n: Int = mark("c"), val g: Unit => Int = u => { println(s"g $u"); 3 })

@main def main(): Unit =
  val k = K()
  println(k.run())
  println(k.run(cb = _ => 2))
  println(k.pair())
  println((Sub(): K).run())
  println(C().g(()))
