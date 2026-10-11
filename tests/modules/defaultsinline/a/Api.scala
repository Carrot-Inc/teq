package dinl

// An inline method's ordinary default getters are methods of its object (`Desugar.addDefaultGetters`):
// the products' class files hold them, so that a downstream module's expansion of an inline
// caller, by teq or by scalac over these products, applies them; an earlier clause's default is
// lifted before a later clause's named arguments; a nested given's default lifts the given found.
object Api:
  def mark(s: String): Int = { print(s); 1 }
  inline def f(a: Int = mark("A"))(b: Int = mark("B")): Int = a + b
  inline def g(a: Int = mark("A"))(b: Int, c: Int = mark("C")): Int = a + b + c
  inline def simple(): Unit = println(f()())
  inline def order(): Unit = println(g()(c = mark("c"), b = mark("b")))
  class GA(val n: Int)
  class GB(val n: Int)
  class GC(val n: Int)
  var count = 0
  given Unit = ()
  given mkA(using Unit): GA = { count += 1; print("A"); new GA(count) }
  given mkB(using a: GA)(using c: GC = { print("C"); new GC(a.n) }): GB = { print("B"); new GB(a.n * 10 + c.n) }
  def use(using b: GB): Unit = println(b.n)
  inline def nested(): Unit = use
