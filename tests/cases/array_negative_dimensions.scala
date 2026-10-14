// A negative length refused where an array is made (`NegativeArraySizeException`, the JVM's `newarray`), and
// `Array.ofDim` of an element type whose erasure is known refusing a negative dimension before it makes any
// array, as dotty's `ArrayConstructors` makes it `Array.newInstance` over every dimension; `fill` and
// `tabulate` of no length are empty, as scala-library's.
object Main:
  def generic[T: scala.reflect.ClassTag](n1: Int, n2: Int): Int = Array.ofDim[T](n1, n2).length
  def main(args: Array[String]): Unit =
    try println(Array.ofDim[Int](0, -1).length)
    catch case _: NegativeArraySizeException => println("negative")
    try println(Array.ofDim[String](0, 2, -3).length)
    catch case e: NegativeArraySizeException => println("negative " + e.getMessage)
    try println(Array.ofDim[Int](-1).length)
    catch case _: NegativeArraySizeException => println("negative")
    try println(new Array[String](-2).length)
    catch case e: NegativeArraySizeException => println("negative " + e.getMessage)
    println(Array.ofDim[Int](2, 3).map(_.length).toList)
    println(generic[Int](0, -1))
    println(Array.fill(-1)(1).length + " " + Array.tabulate(-3)(i => i).length)
