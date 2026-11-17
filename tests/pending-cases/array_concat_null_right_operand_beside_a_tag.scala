// `a ++ n` where `n` is a null array, in a program that names a `ClassTag` value of its own:
// scalac throws a NullPointerException at the concatenation.
// teq is to throw there on every target.
import scala.reflect.ClassTag

object Main {
  def main(args: Array[String]): Unit = {
    println(ClassTag.Int != null)
    println(ClassTag(classOf[String]) != null)
    val a = Array(1, 2)
    val n: Array[Int] = null
    try { println((a ++ n).length) } catch { case e: Throwable => println("thrown: null right operand") }
    try { println((a ++ (null: List[Int])).length) } catch { case e: Throwable => println("thrown: null list") }
  }
}
