// jars: scala-library
// std: lean scala-library
// The kinds of the JVM's arrays: an array is of its element's kind from where it is made, a
// generic call hands it on as it is, a tag that is passed down makes the kind it names, and a
// test against an array type looks at the kind.
import scala.reflect.ClassTag

def same[T](xs: Array[T]): Array[T] = xs
def filled[T: ClassTag](n: Int, x: T): Array[T] = Array.fill(n)(x)
def made[T: ClassTag](n: Int): Array[T] = new Array[T](n)
def down[T: ClassTag](xs: List[T]): Array[T] = collected(xs)
def collected[T: ClassTag](xs: List[T]): Array[T] = xs.toArray
def nested[T: ClassTag](x: T): Array[Array[T]] = Array(Array(x))
def kind(x: Any): String = x match
  case _: Array[Int] => "ints"
  case _: Array[Double] => "doubles"
  case _: Array[Char] => "chars"
  case _: Array[Long] => "longs"
  case _: Array[Boolean] => "booleans"
  case _: Array[Byte] => "bytes"
  case _: Array[String] => "strings"
  case _: Array[Array[Int]] => "arrays of ints"
  case _: Array[?] => "an array of " + x.getClass.getComponentType.getSimpleName
  case _ => "no array"

@main def run(): Unit =
  println(kind(Array(1, 2)) + ", " + kind(Array(1.5)) + ", " + kind(Array('c')) + ", " + kind(Array(1L)) + ", " + kind(Array(true)) + ", " + kind(Array("s")) + ", " + kind(Array[Any](1, "s")) + ", " + kind(Array(Some(1))) + ", " + kind("s"))
  println(kind(same(Array(1))) + ", " + kind(same(Array("s"))) + ", " + kind(filled(1, 2)) + ", " + kind(filled(1, "s")) + ", " + kind(filled(1, 2.5)) + ", " + kind(made[Int](1)) + ", " + kind(made[String](1)) + ", " + kind(made[Option[Int]](1)))
  println(kind(down(List(1))) + ", " + kind(down(List("s"))) + ", " + kind(down(List('c'))) + ", " + kind(nested(1)) + ", " + kind(nested("s")) + ", " + kind(nested(1)(0)))
  println(kind(List(1, 2).toArray) + ", " + kind(Vector("a").toArray) + ", " + kind(Set(1.5).toArray) + ", " + kind(List[Any](1).toArray) + ", " + kind(Map(1 -> 2).toArray) + ", " + kind("xy".toArray) + ", " + kind("a b".split(" ")) + ", " + kind("ab".getBytes))
  val ints = Array(3, 1, 2)
  println(kind(ints.map(_ + 1)) + ", " + kind(ints.map(_.toString)) + ", " + kind(ints.map(_ * 1.5)) + ", " + kind(ints.filter(_ > 1)) + ", " + kind(ints.reverse) + ", " + kind(ints.sorted) + ", " + kind(ints.take(1)) + ", " + kind(ints.clone()) + ", " + kind(ints ++ Array(1)) + ", " + kind(ints :+ 1) + ", " + kind(ints.zip(ints)) + ", " + kind(ints.zipWithIndex))
  println(kind(Array.empty[Double]) + ", " + kind(Array.ofDim[Char](1)) + ", " + kind(Array.tabulate(2)(_.toLong)) + ", " + kind(Array.range(0, 2)) + ", " + kind(Array.copyOf(ints, 5)) + ", " + kind(Array.copyOf(Array("s"), 2)) + ", " + kind(IArray(1, 2)) + ", " + kind(IArray(1, 2).map(_.toString)))
  val through: Any = List("a", "b").toArray
  println(through.asInstanceOf[Array[String]].mkString + " " + Array(Array(1, 2), Array(3)).getClass.getSimpleName + " " + Array(1).getClass.getSimpleName + " " + Array("s").getClass.getSimpleName + " " + Array[Any]().getClass.getSimpleName)
  val builder = Array.newBuilder[Int]
  builder += 1
  builder ++= List(2, 3)
  println(kind(builder.result()) + ", " + kind((Array.newBuilder[String] += "s").result()) + ", " + kind(scala.collection.mutable.ArrayBuffer(1, 2).toArray) + ", " + kind(scala.collection.mutable.ArrayBuffer("a").toArray))
  val halves = scala.collection.mutable.ArrayBuilder.make[Double]
  halves += 1.5
  val longs = new scala.collection.mutable.ArrayBuilder.ofLong
  longs += 1L
  longs ++= Array(2L, 3L)
  println(kind(halves.result()) + ", " + kind(longs.result()) + ", " + kind(Array.newBuilder[Array[Int]].addOne(Array(1)).result()) + ", " + kind(Array.newBuilder[Any].result()))
