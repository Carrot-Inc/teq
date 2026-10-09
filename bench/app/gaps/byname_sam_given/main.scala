package probe.bynamesam
trait Dec[A]:
  def dec(s: String): A
object Dec:
  given int: Dec[Int] = s => s.toInt
  given list[A](using d: => Dec[A]): Dec[List[A]] = s => s.split(",").toList.map(d.dec)
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Dec[List[Int]]].dec("1,2"))
