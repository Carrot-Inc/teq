// `super` in a local lazy val's initialiser forced from another class: dotty's LambdaLift lifts the initialiser into
// the declaring class, where the call stays an `invokespecial` of that class's parent; teq writes the initialiser and
// the accessor there too (`Gen::lazy_method`), and the reader calls them through the holder. Through a class's
// superclass, a qualified `super[A]`, a mixed-in trait, inside a trait's own method (its super accessor), an object's,
// a lambda in the initialiser, the declaring method reading the val too, and a hierarchy whose classes each declare
// one (no accessor of theirs to clash).
import java.util.concurrent.Callable
class Parent { def value: Int = 21; def twice(n: Int): Int = n * 2 }
trait T { def value: Int = 31; def label(s: String): String = s"T $s" }
class Outer extends Parent {
  override def value: Int = 42
  override def twice(n: Int): Int = n * 3
  def viaLazy(): Int = {
    lazy val x = super.value + super.twice(1)
    new Callable[Int] { def call(): Int = x }.call()
  }
  def ownRead(): Int = {
    lazy val x = super.value
    x + new Callable[Int] { def call(): Int = x }.call()
  }
}
class Qualified extends Parent {
  override def value: Int = 43
  def go(): Int = {
    lazy val x = super[Parent].value
    new Callable[Int] { def call(): Int = x }.call()
  }
}
class Mixer extends T {
  override def value: Int = 52
  override def label(s: String): String = s"Mixer $s"
  def viaLazy(): String = {
    lazy val x = super.label("lazy") + super.value
    new Callable[String] { def call(): String = x }.call()
  }
}
trait U extends T {
  override def value: Int = 62
  def go(): Int = {
    lazy val x = super.value
    new Callable[Int] { def call(): Int = x }.call()
  }
}
class UImpl extends U
object Obj extends Parent {
  override def value: Int = 72
  def viaLazy(): Int = {
    lazy val x = super.value
    new Callable[Int] { def call(): Int = x }.call()
  }
}
class Lam extends Parent {
  override def value: Int = 82
  def viaLazy(): Int = {
    lazy val x = List(1).map(_ => super.value).head
    new Callable[Int] { def call(): Int = x }.call()
  }
}
class HA { def value: Int = 1 }
class HB extends HA {
  override def value: Int = 2
  def b(): Int = { lazy val x = super.value; new Callable[Int] { def call(): Int = x }.call() }
}
class HC extends HB {
  override def value: Int = 3
  def c(): Int = { lazy val x = super.value; x }
}
object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer
    println(List(o.viaLazy(), o.ownRead(), new Qualified().go()))
    println(new Mixer().viaLazy())
    println(new UImpl().go())
    println(Obj.viaLazy())
    println(new Lam().viaLazy())
    val hc = new HC
    println(List(hc.b(), hc.c()))
