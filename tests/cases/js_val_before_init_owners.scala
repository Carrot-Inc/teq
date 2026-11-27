// Vals read before their initialisers run, in each kind of owner: a trait mixed into two classes, a parent read by a
// val, a lazy val and a case class parameter, a class with a secondary constructor, two objects, an enum case, a
// class nested in a trait, a local class and an anonymous one.
trait T { val early: Int = read; def read: Int = later; val later: Int = 21 }
class C extends T
class D extends T
abstract class P { def x: Int; println(x) }
class ValChild extends P { val x: Int = 21 }
class LazyChild extends P { lazy val x: Int = { println("lazy"); 21 } }
abstract class CP { def value: Int; println(value) }
case class CC(value: Int) extends CP
class S() { val early: Int = read; def read: Int = later; var later: Int = 21; def this(n: Int) = { this(); later = n } }
object O1 { val early: Int = O2.x; val x: Int = 21 }
object O2 { val x: Int = O1.x }
enum E { case One; val early: Int = read; def read: Int = later; val later: Int = 21 }
trait Nest { class Inner { val early: Int = read; def read: Int = later; val later: Int = 21 } }
class NestOwner extends Nest
object Main { def main(args: Array[String]): Unit = {
  println((new C).early); println((new D).early)
  new ValChild; new LazyChild; new CC(7)
  val s = new S(3); println(s.early); println(s.later)
  println(O1.early); println(E.One.early)
  class Local { val early: Int = read; def read: Int = later; val later: Int = 21 }
  println((new Local).early)
  println((new T {}).early)
  val n = new NestOwner; println((new n.Inner).early)
} }
