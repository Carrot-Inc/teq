// An anonymous class of an empty body is made as its parent's creation, and stays the subclass it stands
// for (tests/errors/constructor_access_anonymous holds what is refused). What is right: a protected
// constructor served where the parent's type is written whole, where the expected type fixes a bare
// parent's type arguments (the class itself, a base of it, a parameter's type), and where the call is
// typed ahead inside another subclass of the parent (a class nested there, a creation, a lambda); a
// protected constructor's default taken inside its package, an inner package and its qualifier; a self
// type the parent's instance has; a sealed class of the same source. An expected type fixes a parameter by a
// type, by a wildcard's bound that says something, by a variable an earlier argument bounded.
package store:
  class Bin[T] protected () { def v: Int = 7 }
  class Box[T] protected (val x: T)
  class Cov[+T] protected () { def v: Int = 3 }
  trait Base[T] { def b: Int = 5 }
  class Kid[T] protected () extends Base[T]
  class Dflt protected (val a: Int = 1)
  class Dflt2 protected (val a: Int)(val b: Int = a + 1)
  class Wide[T] protected[store] (val x: T, val y: Int = 9)
  trait Need
  class SelfOk(val x: Int) extends Need { self: Need => }
  trait Tr { self: Need => }
  class ViaTr(val x: Int) extends Tr with Need
  sealed class Sea(val x: Int)
  class Near extends Bin[Int]:
    class N1 extends Bin
    object N7 extends Bin
    def n3 = new Bin {}
    def n5 = new Bin() {}
    def lam = () => new Bin {}
    def all: Int = new N1().v + N7.v + n3.v + n5.v + lam().v
  object Side:
    def d1 = new Dflt {}
    def d2 = new Dflt() {}
    class D3 extends Dflt
    def d4 = new Dflt2(4)() {}
    def w1 = new Wide("w") {}
    class W2 extends Wide(2)
    def s1 = new Sea(6) {}
  package inner:
    object Deep:
      def i1 = new store.Dflt {}
      class I2 extends store.Dflt()
package shop:
  import store.*
  class Far extends Bin[Int]:
    class F1 extends Bin
    def f3 = new Bin {}
    def f5 = new Bin() {}
    def all: Int = new F1().v + f3.v + f5.v
  // The class is the subclass in the arguments of its own self call and parent call too.
  class Arg(val y: Any) extends Bin[Int]:
    def this() = this(new Bin {})
    def this(n: Int) = this(List(new Bin() {}))
  class Arg2 extends Box[Any](new Box(1) {})
  object Use:
    def take(b: Bin[String]): Int = b.v
    def both[A](a: A, b: Bin[A]): Int = b.v
    def all: List[Int] = List(
      (new Bin[Int] {}).v,
      (new Bin[Int]() {}).v,
      { val a: Bin[Int] = new Bin {}; a.v },
      take(new Bin {}),
      { val c: Cov[Int] = new Cov {}; c.v },
      { val b: Base[Int] = new Kid {}; b.b },
      (new Box[Int](3) {}).x,
      (new Dflt(2) {}).a,
      (new Dflt2(1)(2) {}).b,
      (new SelfOk(8) {}).x,
      (new ViaTr(9) {}).x,
      both(1, new Bin {}),
      { val w: Bin[? <: AnyRef] = new Bin {}; w.v },
      { val c: Cov[AnyRef] = new Cov {}; c.v }
    )
@main def Test(): Unit =
  println(new store.Near().all)
  println(new shop.Far().all)
  println(List(new shop.Arg().y.asInstanceOf[store.Bin[?]].v, new shop.Arg(1).y.asInstanceOf[List[store.Bin[?]]].head.v, new shop.Arg2().x.asInstanceOf[store.Box[Int]].x))
  println(shop.Use.all)
  println(List(store.Side.d1.a, store.Side.d2.a, new store.Side.D3().a, store.Side.d4.b, store.Side.w1.y, new store.Side.W2().x, store.Side.s1.x))
  println(List(store.inner.Deep.i1.a, new store.inner.Deep.I2().a))
