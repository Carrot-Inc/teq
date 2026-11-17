// The tag of an array's `++` made from another tag's class, and one made from a class the program
// computes: neither is a tag the compiler makes, and each is evaluated.
import scala.reflect.ClassTag

class Ringing extends ClassTag[Int] {
  def runtimeClass: Class[?] = { println("class"); classOf[Int] }
}

def twice[T](a: Array[T])(using tag: ClassTag[T]): Int =
  a.++(a)(using ClassTag[T](tag.runtimeClass)).length

object Main {
  def main(args: Array[String]): Unit = {
    println(twice(Array(1))(using new Ringing))
    println(Array(1).++(Array(2))(using ClassTag[Int]({ println("written"); classOf[Int] })).length)
  }
}
