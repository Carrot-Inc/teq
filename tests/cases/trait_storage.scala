// A class's storage takes no name a member of a class extending it uses, nor one of a trait or class in that class's
// linearization (`Constructors.checkNoFieldClashes`, every storage of one object apart from its members): a trait's
// member spelled like the base's copy of a parameter (`$s`) that a subclass implements, an operator (`+`, `$plus`)
// beside the copy of a parameter `plus`, a trait's val of that spelling, a class two levels below. The members keep
// their names, which a call through the trait reads.
trait T:
  def $s: Int
trait Plus:
  def +(k: Int): Int
trait TV:
  val $s: Int = 7
class A(val s: Int):
  val saved = s
class B(override val s: Int) extends A(s) with T:
  def $s: Int = 41
class C(override val s: Int) extends A(s + 1) with TV
class D(s0: Int) extends B(s0)
class P(val plus: Int):
  val saved = plus
class Q(override val plus: Int) extends P(plus + 1) with Plus:
  def +(k: Int): Int = k + 100

@main def run(): Unit =
  val b = B(2)
  val t: T = b
  println(s"${b.saved} ${b.$s} ${t.$s}")
  val c = C(2)
  println(s"${c.saved} ${c.$s}")
  val d = D(5)
  val dt: T = d
  println(s"${d.saved} ${d.$s} ${dt.$s}")
  val q = Q(2)
  val p: Plus = q
  println(s"${q.saved} ${q + 1} ${p + 2}")
