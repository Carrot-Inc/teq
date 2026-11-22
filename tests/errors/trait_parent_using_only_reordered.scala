// A trait of using clauses alone applied without its empty clause by a call that binds its arguments first: named
// arguments out of their order that are no constructor calls (a block, a method's call), or one left out before a
// written one in its second clause: E100, as scalac refuses it.
// expect: 18:18: error: constructor T2 in trait T2 must be called with () argument
// expect: 19:18: error: constructor T2 in trait T2 must be called with () argument
// expect: 20:18: error: constructor T2 in trait T2 must be called with () argument
// expect: 21:18: error: constructor T2 in trait T2 must be called with () argument
// expect: 22:18: error: constructor T3 in trait T3 must be called with () argument
class A(val s: String)
class B(val s: String)
class C(val s: String)
trait T2(using a: A, b: B, c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
trait T3(using a: A)(using b: B, c: C):
  def show = s"${a.s} ${b.s} ${c.s}"
given B = B("gb")
def mk(s: String) = { println(s); C(s) }
class R2 extends T2(using c = { println("c"); C("wc") }, a = A("wa"), b = B("wb"))
class R3 extends T2(using c = mk("wc"), a = A("wa"), b = B("wb"))
class R4 extends T2(using a = A("wa"), c = { println("c"); C("wc") }, b = B("wb"))
class D2 extends T2(using c = { println("c"); C("wc") }, a = { println("a"); A("wa") }, b = { println("b"); B("wb") })
class D3 extends T3(using A("wa"))(using c = C("wc"))
