// A class tag the program writes itself is an argument like another: evaluated at each call that
// takes it, whether the member reads it or not.
import scala.reflect.ClassTag

class Crate(val n: Int)

def loudTag: ClassTag[Crate] = { println("tag made"); ClassTag(classOf[Crate]) }

object Main {
  def main(args: Array[String]): Unit = {
    println("start")
    val a = Array(new Crate(1), new Crate(5))
    val b = Array(new Crate(2))
    println(a.++(b)(using loudTag).length)
    println(a.map(c => new Crate(c.n + 1))(using loudTag).length)
    println(a.:+(new Crate(3))(using loudTag).length)
    println(a.++(List(new Crate(4)))(using loudTag).length)
  }
}
