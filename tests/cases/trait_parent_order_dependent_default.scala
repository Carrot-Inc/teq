// A trait of evidence alone whose default reads an earlier argument of the parent's call: the call's bindings are made
// once, in the order scalac makes them, and the trait's fields read them (the default getter takes the binding of the
// argument it reads). A default reading one earlier argument, two, a searched one, a default in an implicit clause after
// the using one, and the same parent written as an anonymous class's.
class A(val s: String)
class B(val s: String)
class C(val s: String)
class X(val s: String)
given ga(using DummyImplicit): A = { println("given a"); A("ga") }
given gc(using DummyImplicit): C = { println("given c"); C("gc") }
trait T1(using a: A)(using b: B = { println("default " + a.s); B(a.s) }, c: C):
  def show = a.s + " " + b.s + " " + c.s
trait T2(using a: A, x: X)(using b: B = { println("default " + a.s + x.s); B(a.s + x.s) }, c: C):
  def show = a.s + " " + x.s + " " + b.s + " " + c.s
trait T3(using a: A)(using b: B = { println("default " + a.s); B(a.s) }, c: C):
  def show = a.s + " " + b.s + " " + c.s
trait T4(using a: A)(implicit b: B = { println("default " + a.s); B(a.s) }, c: C):
  def show = a.s + " " + b.s + " " + c.s
class S1 extends T1(using { println("a"); A("wa") })(using c = { println("c"); C("wc") })()
class S2 extends T2(using { println("a"); A("wa") }, { println("x"); X("wx") })(using c = { println("c"); C("wc") })()
class S3 extends T3
class S4 extends T4(using { println("a"); A("wa") })(using c = { println("c"); C("wc") })
object Main:
  def main(args: Array[String]): Unit =
    println(new S1().show)
    println(new S2().show)
    println(new S3().show)
    println(new S4().show)
    println((new T1(using { println("a"); A("wa") })(using c = { println("c"); C("wc") })() {}).show)
