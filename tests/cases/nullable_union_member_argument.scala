// A member of a class's parent selected on a union with Null (`Queue[A] | Null`, as scala-library writes a nullable
// field): the parent's type argument is the class's, the union's base type the non-null side's.
import scala.collection.mutable
class Leading[A]:
  private var lookahead: mutable.Queue[A] | Null = null
  def add(a: A): Unit =
    if lookahead == null then lookahead = new mutable.Queue[A]
    lookahead.addOne(a)
  def size: Int = if lookahead == null then 0 else lookahead.length
object Main:
  def main(args: Array[String]): Unit =
    val l = new Leading[Int]
    l.add(1); l.add(2)
    println(l.size)
