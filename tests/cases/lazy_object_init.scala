// Lazy vals read while their object is being initialised: the object's own constructor reads its lazy val (dotty
// looks the VarHandle up in the module class's static initialiser before the instance is made), a lazy val of a
// class read in its constructor, and top-level definitions of a file: a constant (Memoize's getter of the literal,
// no field), a lazy val and a given alias read once, and a derived given.
object Early:
  var log = List.empty[String]
  lazy val value: String = { log = "computed" :: log; "early value" }
  val copy: String = value + " read in the constructor"
  log = "constructed" :: log

class ReadsInConstructor:
  lazy val l: Int = 21
  val doubled: Int = l * 2

final val answer = 42
var topRuns = 0
lazy val topLazy: String = { topRuns += 1; "top lazy" }
given topGiven: List[Int] = { topRuns += 1; List(1, 2) }

case class Point(x: Int, y: Int) derives CanEqual

object Main:
  def main(args: Array[String]): Unit =
    println(Early.copy)
    println(Early.log.reverse.mkString(", "))
    println(new ReadsInConstructor().doubled)
    println(answer + 1)
    println(topLazy + " " + topLazy)
    println(summon[List[Int]])
    println(summon[List[Int]].sum)
    println("runs " + topRuns)
    println(Point(1, 2) == Point(1, 2))
