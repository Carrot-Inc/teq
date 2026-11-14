// A class that takes, beside `Comparable`'s `compareTo`, a trait's `compareTo(o: Any)` named apart by a
// `@targetName` answers each by its own, directly and through either type: the lines scalac 3.8.4 prints.
import scala.annotation.targetName
trait Extra { @targetName("compareAny") def compareTo(o: Any): Int = 7 }
class B(val n: Int) extends Extra with Comparable[B] { def compareTo(b: B): Int = n - b.n }
object Main {
  def main(args: Array[String]): Unit = {
    val a = new B(1); val b = new B(3)
    println(a.compareTo(b))
    println(a.compareTo("x": Any))
    println((a: Comparable[B]).compareTo(b))
    println((a: Extra).compareTo(b))
  }
}
