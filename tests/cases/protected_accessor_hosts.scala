// A protected member reached from outside its package needs an accessor, which an enclosing class that derives from
// the member's class holds (dotty's ProtectedAccessors, AccessProxies' `hostForAccessorOf`): from an inner class, an
// anonymous class or a lambda of a subclass, and from a protected member's companion; from the member's own package
// no accessor is needed. A parent call's arguments stand in the constructor of the class they build, an anonymous
// subclass's too, with an empty body or a body, its parent's type arguments written or inferred. A member
// `@publicInBinary` needs no accessor (`needsAccessorIfNotInSubclass`), whatever the annotation's spelling: a def, a
// var, a class parameter, a conversion and a given. Each runs (tests/errors/protected_accessor_without_host.scala and
// tests/errors/protected_accessor_not_public_in_binary.scala refuse those with no host).
import scala.annotation.publicInBinary
import scala.annotation.publicInBinary as pib
package lib {
  import scala.language.implicitConversions
  class Tag(val n: Int)
  class C(val origin: String) {
    protected def m(x: Int): String = "m" + x + origin
    @publicInBinary protected def pb(x: Int): String = "pb" + x + origin
    @pib protected var w: Int = 1
    @publicInBinary protected implicit def mark(i: Int): Tag = Tag(i + 10)
    @publicInBinary protected given conv: Conversion[Boolean, Tag] = b => Tag(if b then 1 else 0)
    override def toString: String = origin
  }
  class Held(val origin: String, @publicInBinary protected val v: Int)
  class Box[T](val item: T) {
    protected def m(x: Int): String = "bm" + x
    override def toString: String = "box" + item
  }
  class Obj2 { def peek: String = Obj2.hidden }
  object Obj2 { protected def hidden: String = "hidden" }
  trait Same { self: C => def f: String = m(0) }
  class Both(o: String) extends C(o) with Same
}
package use {
  import scala.language.implicitConversions
  class Sub extends lib.C("sub") {
    class In { def f: String = m(4) }
    def inner: String = new In().f
    def lambda: String = List(5).map(m).head
    def anon: String = new Runnable { def run(): Unit = (); override def toString: String = m(6) }.toString
  }
  object Peek { def p: String = new lib.Obj2().peek }
  trait Side { self: lib.C =>
    def binary: String = { w = w + 2; val t: lib.Tag = 5; val u: lib.Tag = true; pb(7) + w + t.n + u.n }
    def parent: lib.C = new lib.C(m(1)) {}
    def lambda: lib.C = new lib.C((() => m(2))()) { def extra: Int = 1 }
  }
  trait HeldSide { self: lib.Held => def hv: String = "v" + v + origin }
  trait BoxSide { self: lib.Box[Int] =>
    def again: lib.Box[String] = new lib.Box(m(3)) {}
    def typed: lib.Box[String] = new lib.Box[String](m(4)) { def extra: Int = 1 }
  }
  class BoxBoth extends lib.Box(5) with BoxSide
  class Both extends lib.C("b") with Side
  class HeldBoth extends lib.Held("h", 8) with HeldSide
  @main def run(): Unit =
    val s = new Sub
    println(s.inner)
    println(s.lambda)
    println(s.anon)
    println(Peek.p)
    println(new lib.Both("same").f)
    val b = new Both
    println(b.binary)
    println(List(b.parent, b.lambda).mkString(","))
    println(new HeldBoth().hv)
    val x = new BoxBoth
    println(List(x.again, x.typed).mkString(","))
}
