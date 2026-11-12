// Local lazy vals in a class's constructor trees: an auxiliary constructor's body and its self call's argument, a
// superclass's named arguments written out of their order (their temporaries run first, in the written order), a
// trait's named arguments, the primary constructor's body statements, beside a member and a method. Each is a
// definition the lazy-name walk and the declaring-class walk meet (dotty's MegaPhase visits the parents, then the
// body with the secondary constructor at its place), its accessor and initialiser in the declaring class.
class Base(val a: Int, val b: Int)
trait T(val t: Int, val u: Int)

class C(p: Int)
    extends Base(b = { lazy val x = { println("b"); p + 2 }; x }, a = { lazy val x = { println("a"); p + 1 }; x })
    with T(u = { lazy val x = { println("u"); 7 }; x }, t = { lazy val x = { println("t"); 8 }; x }):
  lazy val x = 10
  println({ lazy val x = { println("body"); 20 }; x + x })
  { lazy val y = 21; println("block " + (y + y)) }
  def this() =
    this({ lazy val x = { println("self call"); 30 }; x })
    lazy val x = { println("aux"); 40 }
    println(x + x)
  def m(): Int = { lazy val x = 50; x }

class D extends Base(b = { lazy val y = 2; y }, a = { lazy val x = 1; x })

// Initialisers in a superclass call and an auxiliary constructor reading the enclosing instance or a top-level def,
// from an inner class (tests/cases/lazy_local_constructor_homes has the named arguments that read it).
class Outer:
  val k = 5
  def f = 6
  class Inner extends Base({ lazy val x = k + f; x }, 0)
  class Inner3 extends Base(1, 0):
    def this(s: String) =
      this()
      lazy val z = s.length + k
      println(z)

def top = 8
class TopUser extends Base({ lazy val x = top; x }, 0)

@main def run(): Unit =
  val c = new C()
  println(c.a + c.b + c.x + c.m() + c.t + c.u)
  val c2 = new C(100)
  println(c2.a + c2.b)
  val d = new D
  println(d.a)
  println(d.b)
  val o = new Outer
  println(new o.Inner().a)
  new o.Inner3("abc")
  println(new TopUser().a)
