// The `ClassTag` evidence of the array-making members is evaluated once, in its argument's place, and
// makes the array: `Array.fill`, `tabulate`, `empty`, `ofDim` (the outer arrays by the tag's `wrap`) and
// an array's `map` call the tag's `newArray` first, then fill it, as scala-library's do. A program's own
// tag is the one asked; the standard ones (`ClassTag.Int`, the compiler's) make the array a program
// cannot tell apart. (A collection's `toArray`, which classes of libraries override, keeps its tag erased
// outside the JVM: docs/COMPATIBILITY.md.)
// `ClassTag.Nothing` and `Null` are scala-library's, of the runtime classes `classOf[Nothing]` and
// `classOf[Null]` name.
import scala.reflect.ClassTag

class Logged[T](name: String, base: ClassTag[T]) extends ClassTag[T]:
  def runtimeClass: Class[?] = base.runtimeClass
  override def newArray(n: Int): Array[T] = { println(s"$name.newArray($n)"); base.newArray(n) }

object Main:
  var count = 0
  def tag: ClassTag[Int] = { count += 1; ClassTag.Int }
  val ints = new Logged[Int]("ints", ClassTag.Int)
  val strings = new Logged[String]("strings", ClassTag(classOf[String]))

  def main(args: Array[String]): Unit =
    println(Array.fill[Int](2)(7)(using tag).mkString(",") + " " + count)
    println(Array.fill[Int](2)({ println("elem"); 1 })(using ints).toList)
    println(Array.tabulate[String](2)(i => { println(s"f($i)"); i.toString })(using strings).toList)
    println(Array.empty[Int](using ints).length)
    println(Array(1, 2, 3).map(x => { println(s"map $x"); x * 2 })(using ints).toList)
    println(Array[String]("x", "y")(using strings).length)
    println((Array("a") ++ Array("b"))(using strings).toList)
    println(Array("a").appended("b")(using strings).toList)
    println(Array.ofDim[Int](2, 3).map(_.toList).toList)
    println(Array.ofDim[Int](2, 2)(using ints).length)
    println(Array.ofDim[Int](1, 1, 2).map(_.map(_.toList).toList).toList)
    println(List(classOf[Nothing].getName, classOf[Null].getName))
    println(List(ClassTag.Nothing, ClassTag.Null, ClassTag.Nothing.runtimeClass.getName))
