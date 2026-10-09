// A stable identifier pattern compares a value class as its box, `pattern == scrutinee` (dotty's EqualTest,
// PatternMatcher.scala 811-812): the box's equals, a user's included.
class V(val u: Int) extends AnyVal
case class C(u: Int) extends AnyVal
class T(val u: Int) extends AnyVal { override def equals(o: Any): Boolean = true }
object Main {
  val key: Any = (new V(1): Any)
  val cv: Any = C(1)
  val tk: Any = new T(5)
  val k: V = new V(3)
  def main(args: Array[String]): Unit = {
    println(new V(1) match { case `key` => "yes"; case _ => "no" })
    println(new V(2) match { case `key` => "yes"; case _ => "no" })
    println(C(1) match { case `cv` => "yes"; case _ => "no" })
    println(new T(9) match { case `tk` => "yes"; case _ => "no" })
    println(new V(3) match { case `k` => "yes"; case _ => "no" })
  }
}
