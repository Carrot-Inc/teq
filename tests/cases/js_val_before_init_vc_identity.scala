// Two reads of one value class field before its initialiser runs: each read boxes the zero anew, so the two are not
// the same instance.
class V(val x: Int) extends AnyVal
class A {
  def get: V = later
  val first: AnyRef = get.asInstanceOf[AnyRef]
  val second: AnyRef = get.asInstanceOf[AnyRef]
  val later: V = new V(21)
}
object Main { def main(args: Array[String]): Unit = {
  val a = new A
  println(a.first eq a.second)
} }
