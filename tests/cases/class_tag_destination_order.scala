// An array made by a program's own tag before its elements are computed, each written into it in turn and a
// source element read when its turn comes, as scala-library's `Array.tabulate` and `ArrayOps.map` do: a
// computation that reads the destination or changes the source sees it.
import scala.reflect.ClassTag
object Main:
  var dest: Array[Int] = Array.empty[Int]
  val tag = new ClassTag[Int]:
    def runtimeClass = classOf[Int]
    override def newArray(n: Int): Array[Int] =
      dest = new Array[Int](n)
      dest
  def main(args: Array[String]): Unit =
    println(Array.tabulate[Int](3)(i => if i == 0 then 7 else dest(i - 1) + 1)(using tag).toList)
    val src = Array(1, 2)
    println(src.map(x => { src(1) = 9; x })(using tag).toList)
