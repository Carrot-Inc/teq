// `a ++ n` where `n` is a null array: scalac throws a NullPointerException, as for every other
// operation on a null array. The program names no `ClassTag` value.
// teq is to throw there on every target.
object Main {
  def main(args: Array[String]): Unit = {
    val a = Array(1, 2)
    val n: Array[Int] = null
    try { println((a ++ n).length) } catch { case e: Throwable => println("thrown: null right operand") }
    try { println((a ++ (null: List[Int])).length) } catch { case e: Throwable => println("thrown: null list") }
  }
}
