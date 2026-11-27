// Fields read before their initialisers run, one of each zero: Int, Long, Char, Boolean, Unit and a union; a
// generic field at Int and at a value class.
class V(val x: Int) extends AnyVal
class G[T](v: T) { val early: T = later; val later: T = v }
class A {
  def readI: Int = i
  def readL: Long = l
  def readC: Char = c
  def readB: Boolean = b
  def readU: Unit = u
  def readUnion: Int | String = union
  println(readI + 1)
  println(readL + 1L)
  println(readC.toInt)
  println(readC == 0)
  println(if (readB) "yes" else "no")
  println(readU)
  println(readUnion)
  val i: Int = 21
  val l: Long = 21L
  val c: Char = 'x'
  val b: Boolean = true
  val u: Unit = ()
  val union: Int | String = 21
}
object Main { def main(args: Array[String]): Unit = {
  new A
  println((new G[Int](21)).early)
  println(((new G[V](new V(21))).early: Any) == null)
} }
