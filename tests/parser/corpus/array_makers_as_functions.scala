// A member that makes an array, taken as a function value: its `ClassTag` is found where the
// method becomes the function, from the type argument written or the type expected.
import scala.reflect.ClassTag

def filler[T: ClassTag](n: Int): (=> T) => Array[T] = Array.fill[T](n)
def mapper[T: ClassTag](a: Array[Int]): (Int => T) => Array[T] = a.map[T]

@main def run(): Unit =
  val a = Array(1, 2, 3)
  val strings: (Int => String) => Array[String] = a.map
  println(strings(_.toString).mkString(","))
  val doubles = a.map[Double]
  println(doubles(_ * 1.5).map(_.toInt).mkString(","))
  val fill: (=> String) => Array[String] = Array.fill(2)
  println(fill("x").mkString)
  val tabulate = Array.tabulate[Int](3)
  println(tabulate(_ * 2).mkString(","))
  println(filler[Long](2)(7L).mkString(",") + " " + filler[String](1)("s").mkString + " " + mapper[Char](a)(i => ('a' + i).toChar).mkString)
  println(List(1, 2).map(Array.ofDim[Int]).map(_.length).mkString(","))
  val flat: (Int => List[Int]) => Array[Int] = a.flatMap
  println(flat(i => List(i, i)).mkString(","))
