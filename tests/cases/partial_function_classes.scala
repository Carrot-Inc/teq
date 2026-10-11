// A class extending PartialFunction: its overrides are reached by ordinary dispatch, as a
// Function1, through isDefinedAt and applyOrElse, and by the library's operations over it, with
// each call's effect once.
var calls = 0
class PF(val offset: Int) extends PartialFunction[Int, Int]:
  def isDefinedAt(x: Int): Boolean = { calls += 1; println(s"defined $x"); x > 0 }
  def apply(x: Int): Int = { println(s"apply $x"); x + offset }
class Child extends PF(10):
  override def isDefinedAt(x: Int): Boolean = x > 1 && super.isDefinedAt(x)
class Own(n: Int) extends PartialFunction[Int, String]:
  val label = s"own$n"
  def isDefinedAt(x: Int): Boolean = x % 2 == 0
  def apply(x: Int): String = s"$label:$x"
  override def applyOrElse[A1 <: Int, B1 >: String](x: A1, default: A1 => B1): B1 =
    println(s"applyOrElse $x")
    if x % 2 == 0 then apply(x) else default(x)
abstract class Named[A, B] extends PartialFunction[A, B]:
  def name: String
  override def toString: String = s"Named($name)"
object Evens extends Named[Int, Int]:
  def name: String = "evens"
  def isDefinedAt(x: Int): Boolean = x % 2 == 0
  def apply(x: Int): Int = x / 2

@main def run(): Unit =
  val p: PartialFunction[Int, Int] = new Child
  val f: Int => Int = p
  println(f(2))
  println(p.isDefinedAt(1))
  println(p.applyOrElse(1, (x: Int) => { println("fallback"); 42 }))
  println(p.applyOrElse(5, (x: Int) => { println("fallback"); 42 }))
  println(p.lift(0))
  println(p.lift(3))
  println(List(-1, 2, 3).collect(p))
  val q = new Own(1)
  println(List(1, 2, 3, 4).collect(q))
  println(q.applyOrElse(3, (x: Int) => "default"))
  println(q.orElse(Evens.andThen(_.toString)).isDefinedAt(3))
  println((Evens orElse p)(3))
  println(Evens.andThen(_ + 1)(4))
  println(Evens)
  println(List(1, 2, 3, 4).map(Evens.lift))
  println(p.asInstanceOf[PF].offset)
  println(p match { case _: Child => "child"; case _ => "other" })
  println(f.isInstanceOf[PartialFunction[?, ?]])
  println(calls)
