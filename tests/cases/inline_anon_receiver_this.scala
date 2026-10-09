// A class an inline method of `C` makes reads `C.this` through the receiver's local at the
// expansion, which it captures, also where another class of `C` reads `C.this` through the local
// that stands for it in the classes nested in `C` (the shape of scala/scala3's tests/run/i26176,
// whose private member scalac 3.8.4 reaches through an accessor it does not emit); an anonymous
// class's creation passes its parent the receiver's member, a named class's constructor does,
// and a named class creates itself and names its own type in the body's types.
class C(val n: Int):
  def secret = 42 + n
  def foo = 10
  def bar: Runnable = new Runnable { def run() = println(s"bar $secret") }
  inline def anon: C = new C(n * 3) { override def foo = C.this.secret + this.n }
  inline def local: C =
    class L(k: Int) extends C(n + k) { override def foo = C.this.secret + this.n; def twin: L = new L(k + 1) }
    val l: L = new L(1)
    val ls: List[L] = List(l, l.twin)
    println(ls.map(_.foo).sum)
    l.twin
  inline def parentOnly: C =
    class P(k: Int) extends C(n + k)
    new P(5)

class D(val n: Int):
  def secret = 7 * n
  def foo = 1
  inline def m = new D(n) { override def foo = D.this.secret }

@main def run(): Unit =
  val e = new C(1).local
  e.bar.run()
  println(e.foo)
  println(e.n)
  println(new C(2).anon.foo)
  println(new C(3).parentOnly.n)
  println(new D(2).m.foo)
