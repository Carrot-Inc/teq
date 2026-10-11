// A value class's instance as a stable pattern compares as its box, against a primitive scrutinee too: `1 match {
// case `key` => }` for a `key: V` is `key == 1`, `V`'s box unequal to the number (dotty's `EqualTest`, the operand
// boxed by erasure; master's JVM compared the underlying `Int` and printed `yes`). Two instances of the class
// compare by their values, two numbers as numbers.
class V(val x: Int) extends AnyVal
given CanEqual[V, Int] = CanEqual.derived
given CanEqual[Int, V] = CanEqual.derived
@main def run() =
  val key = new V(1)
  println(1 match { case `key` => "yes"; case _ => "no" })
  val other = new V(1)
  println(other match { case `key` => "same"; case _ => "different" })
  val n = 2
  println(2 match { case `n` => "two"; case _ => "not two" })
  println(new V(2) match { case V2(y) => s"v$y" })
object V2 { def unapply(v: V): Some[Int] = Some(v.x) }
