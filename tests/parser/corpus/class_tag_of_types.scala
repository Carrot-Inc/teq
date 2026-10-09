// The evidence of an array's element: a class type's, a union's, an intersection's, a literal
// type's, and what a generic definition passes on.
import scala.reflect.ClassTag

trait Named:
  def name: String
trait Sized:
  def size: Int
final class Both(val name: String, val size: Int) extends Named, Sized

def fill[T: ClassTag](n: Int, x: T): Array[T] = Array.fill(n)(x)
def collected[T: ClassTag](xs: List[T]): Array[T] = xs.toArray
def mapped[T, U: ClassTag](xs: Array[T])(f: T => U): Array[U] = xs.map(f)
def tagOf[T](using tag: ClassTag[T]): String = tag.toString

@main def run(): Unit =
  val mixed = Array(Array(1), List(1))
  println(mixed.length)
  val union: Array[Int | String] = Array(1, "two")
  println(union.mkString(","))
  val both: Array[Named & Sized] = Array(new Both("b", 2))
  println(both.map(x => x.name + x.size).mkString(","))
  val literal = Array[1 | 2](1, 2)
  println(literal.sum)
  println(fill(3, "x").mkString + collected(List(1, 2, 3)).sum + mapped(Array(1, 3))(_ * 2.5).mkString(","))
  println(Array.empty[String].length + Array.tabulate(3)(i => i * i).mkString(",") + List(1, 2).map(_ + 1).toArray.mkString(","))
  println(tagOf[Int] + " " + tagOf[String] + " " + tagOf[Int | String] + " " + tagOf[Named & Sized] + " " + tagOf[Both])
  println((Array(1, 2) :+ 4).mkString(","))
  println((0 +: Array(1.5, 2.5)).mkString(","))
  println(IArray(1, 2, 3).map(_ + 1).toList)
