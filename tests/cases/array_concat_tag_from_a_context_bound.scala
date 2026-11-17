// The tag of an array's `++` that is the caller's context bound, the tag of an array of arrays
// made from it by `wrap`, and a `wrap` the program writes: each is evaluated where scalac
// evaluates it.
import scala.reflect.ClassTag

class Ringing extends ClassTag[Int] {
  def runtimeClass: Class[?] = classOf[Int]
  override def wrap: ClassTag[Array[Int]] = {
    println("wrap")
    ClassTag(classOf[Array[Int]])
  }
}

def ringing: ClassTag[Int] = { println("given"); new Ringing }
def twice[T: ClassTag](a: Array[T]): Int = (a ++ a).length
def twiceNested[T: ClassTag](a: Array[Array[T]]): Int = (a ++ a).length

object Main {
  def main(args: Array[String]): Unit = {
    println(twice(Array(1))(using ringing))
    println(twiceNested(Array(Array(1)))(using ringing))
    println(Array(Array(1)).++(Array(Array(2)))(using ringing.wrap).length)
  }
}
