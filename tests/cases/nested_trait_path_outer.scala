// A trait nested in a class, mixed in through a path (Scala 3's tests/run/i23245c, i1820,
// noProtectedSuper): the path is the enclosing instance the trait's members read, each trait its
// own parent clause's, resolved where the parent clause stands.
package outers

trait T:
  def f = 42
  trait D:
    lazy val g = f

object C extends T

object Z extends C.D

class A(val name: String):
  trait Inner:
    def show = s"inner of $name"
  trait Other:
    def other = s"other of $name"
  trait Typed[T]:
    def typed = s"typed of $name"
  type Alias = Inner
  // An explicit prefix wins over the enclosing instance; a trait the superclass mixes in keeps
  // the superclass's.
  class Within extends Base with Holder.b.Other
  def within: String =
    val w = new Within
    w.show + ", " + w.other
  class Generic[T] extends Holder.b.Typed[T]
  def generic: String = (new Generic[Int]).typed
  def local(that: A): String =
    class L extends that.Inner
    (new L).show

class Twin(val name: String):
  trait Inner:
    def twin = s"twin of $name"

object Holder:
  val a = new A("a")
  val b = new A("b")
  val t = new Twin("t")

class Outside extends Holder.a.Inner

class Both extends Holder.a.Inner with Holder.b.Other

class Box[T]
class Boxed[T] extends Box[T] with Holder.a.Typed[T]

class Base extends Holder.a.Inner
class Derived extends Base with Holder.b.Other

class SameName extends Holder.a.Inner with Holder.t.Inner

class Aliased extends Holder.b.Alias

class Qualified extends outers.Holder.b.Inner

class Through(val a: A):
  class ViaThis extends this.a.Inner
  class ViaQualified extends Through.this.a.Inner
  def get = (new ViaThis).show + ", " + (new ViaQualified).show

import Holder.a
class Shadowed extends a.Inner:
  val a = Holder.b

class ShadowedByImport extends a.Inner:
  import Holder.{b as a}
  def again = a.name

class B(base: Int):
  protected def foo(): Int = base
  trait BInner:
    def bar() = foo() + 1

@main def m(): Unit =
  println(Z.g)
  println((new Outside).show)
  val both = new Both
  println(both.show + ", " + both.other)
  val derived = new Derived
  println(derived.show + ", " + derived.other)
  println(new A("x").within)
  println(new A("x").generic + ", " + (new Boxed[Int]).typed)
  println(new A("x").local(new A("y")))
  val same = new SameName
  println(same.show + ", " + same.twin)
  println((new Aliased).show)
  println((new Qualified).show)
  println(new Through(new A("z")).get)
  println((new Shadowed).show)
  val imported = new ShadowedByImport
  println(imported.show + ", " + imported.again)
  val b = new B(1)
  val bi = new b.BInner {}
  println(bi.bar())
