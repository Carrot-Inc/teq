// The tag of an array's `++` that is a given of the program, for an opaque element type and for a
// union: a given that takes a parameter is evaluated at each use, so each prints once where its
// array is made and once more at the concatenation; and a class tag that is an array's element is
// an ordinary value.
import scala.reflect.ClassTag

class Tick
given Tick = new Tick

object Marks {
  opaque type Mark = Int
  def mark(i: Int): Mark = i
  given tag(using Tick): ClassTag[Mark] = { println("opaque tag"); ClassTag.Int }
}

object Unions {
  given unionTag(using Tick): ClassTag[Int | String] = { println("union tag"); ClassTag(classOf[Object]) }
}

object Main {
  def main(args: Array[String]): Unit = {
    import Marks.*
    import Unions.given
    val a = Array(mark(1))
    println("made")
    println((a ++ a).length)
    val b = Array[Int | String](1, "s")
    println("made")
    println((b ++ b).length)
    val tags = Array[ClassTag[Int]](ClassTag.Int).appended(ClassTag.Int)
    println(tags(1) != null)
    println(tags(1) == ClassTag.Int)
  }
}
