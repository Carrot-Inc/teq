// An argument that an inline method's expansion puts in the place of its parameter means there
// what it means at the call: a `this` in it is the caller's instance, also where a class the
// body declares reads the parameter: a named class's method, its val's initialiser, its
// constructor statements and its parent's argument, an anonymous class, a lambda's class, an
// object and a given's class; for a field, a by-name parameter's and an inline parameter's
// code and a lambda given by value, by name and to an inline parameter; read at the body's own
// level too; and passed on to another inline method (scalac prints
// `5 2 2 1 6 6 5 2 2 2 2 2 2 11 105`, `23 11 20 10 24 24 23 11 11 11 11 11 11 110 123`, `9`).
trait Fn { def apply(): Int }
trait Marker
class Base(val value: Int)

object Inl:
  inline def method(k: Int): Int =
    class Two { def put(x: Int): Int = x + k }
    val t = new Two
    t.put(1) + t.put(2)
  inline def init(k: Int): Int =
    class Two { val got: Int = k + 1 }
    new Two().got
  inline def ctor(k: Int): Int =
    class Two { var s: Int = 0; s += k; s += k }
    new Two().s
  inline def parent(k: Int): Int =
    class Two extends Base(k) { def put(): Int = value }
    new Two().put()
  inline def anon(k: Int): Int =
    val f = new Fn { def apply(): Int = k + 2 }
    f() + f()
  inline def sam(k: Int): Int =
    val f: Fn = () => k + 2
    f() + f()
  inline def obj(k: Int): Int =
    object Obj { def put(x: Int): Int = x + k }
    Obj.put(1) + Obj.put(2)
  inline def giv(k: Int): Int =
    given g: Marker with { def put(x: Int): Int = x + k }
    g.put(1)
  inline def byName(k: => Int): Int =
    class Two { def put(x: Int): Int = x + k }
    new Two().put(1)
  inline def inl(inline k: Int): Int =
    class Two { def put(x: Int): Int = x + k }
    new Two().put(1)
  inline def fun(k: () => Int): Int =
    class Two { def put(x: Int): Int = x + k() }
    new Two().put(1)
  inline def funByName(k: => () => Int): Int =
    class Two { def put(x: Int): Int = x + k() }
    new Two().put(1)
  inline def funInl(inline k: () => Int): Int =
    class Two { def put(x: Int): Int = x + k() }
    new Two().put(1)
  inline def both(k: Int): Int =
    class Two { def put(): Int = k }
    k * 10 + new Two().put()
  inline def outer(k: Int): Int = method(k) + 100

class K(val n: Int):
  def f(): Int = n
  def lines(): String =
    List(Inl.method(n), Inl.init(this.n), Inl.ctor(n), Inl.parent(n), Inl.anon(n), Inl.sam(n), Inl.obj(n), Inl.giv(n),
      Inl.byName(this.f()), Inl.inl(this.f()), Inl.fun(() => this.n), Inl.funByName(() => this.n), Inl.funInl(() => n),
      Inl.both(n), Inl.outer(n)).mkString(" ")

object Holder:
  val n: Int = 3
  def line(): Int = Inl.method(n)

@main def run(): Unit =
  println(new K(1).lines())
  println(new K(10).lines())
  println(Holder.line())
