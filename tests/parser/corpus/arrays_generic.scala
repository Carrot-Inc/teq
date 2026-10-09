// Arrays through generic code: the array a generic call returns is the one it took, an element
// type that is a parameter comes with its tag, and what is built from an array is a copy.
import scala.reflect.ClassTag

def same[T](xs: Array[T]): Array[T] = xs
def first[T](xs: Array[T]): T = xs(0)
def put[T](xs: Array[T], i: Int, x: T): Unit = xs(i) = x
def count[T](xs: Array[T]): Int = xs.length
def copied[T](xs: Array[T]): Array[T] = xs.clone()
def filled[T: ClassTag](n: Int, x: T): Array[T] = Array.fill(n)(x)
def made[T: ClassTag](n: Int): Array[T] = new Array[T](n)
def down[T: ClassTag](xs: List[T]): Array[T] = collected(xs)
def collected[T: ClassTag](xs: List[T]): Array[T] = xs.toArray
def pairs[T: ClassTag](xs: Array[T]): Array[Array[T]] = xs.map(x => Array(x, x))
def total(xs: Int*): Int = xs.sum
def describe(x: Any): String = x match
  case Array() => "empty"
  case Array(a) => "one " + a
  case Array(a, b, rest*) => "two and " + rest.length + ": " + a + " " + b
  case _ => "no array"

@main def run(): Unit =
  val ints = Array(3, 1, 2)
  val strings = Array("b", "a")
  println((same(ints) eq ints).toString + " " + (same(strings) eq strings) + " " + first(ints) + first(strings) + count(ints) + count(strings))
  put(ints, 0, 9)
  put(strings, 1, "z")
  println(ints.mkString(",") + " " + strings.mkString(","))
  val copy = copied(ints)
  copy(1) = 7
  println(ints.mkString(",") + " " + copy.mkString(",") + " " + (copy eq ints))
  println(filled(2, 'c').mkString + filled(2, 1.5).mkString(",") + filled(2, "s").mkString + filled(1, 4L).mkString + filled(1, true).mkString)
  val zeros = made[Int](2)
  val nulls = made[String](2)
  println(zeros.mkString(",") + " " + (nulls(0) == null) + " " + made[Long](1)(0) + " " + made[Boolean](1)(0) + " " + made[Char](1)(0).toInt)
  println(down(List(1, 2, 3)).sum.toString + down(List("x", "y")).mkString)
  val grid = Array(Array(1, 2), Array(3, 4))
  grid(1)(0) = 8
  println(grid.map(_.mkString("[", ",", "]")).mkString + " " + grid.length + grid(0).length)
  val table = Array.ofDim[Int](2)
  table(1) = 5
  println(table.mkString(",") + " " + pairs(Array(1, 2)).map(_.sum).mkString(",") + " " + pairs(Array("a")).head.mkString)
  val any: Array[Any] = Array(1, 'c', "s", 2L, true, 1.5, ())
  println(any.map(x => x.toString).mkString(" ") + " " + any.length)
  any(0) = "changed"
  println(any(0).toString + " " + first(any) + " " + same(any).length)
  println(total() + total(1) + total(1, 2, 3))
  println(describe(ints) + "; " + describe(Array("p", "q", "r", "s")) + "; " + describe(Array[Int]()) + "; " + describe(Array(1.5)) + "; " + describe(List(1)))
  val wrapped = scala.collection.immutable.ArraySeq.unsafeWrapArray(ints)
  ints(2) = 100
  println(wrapped.mkString(",") + " " + wrapped.length)
  println(List(1, 2).toArray.map(_ + 1).toList.toString + Vector("a").toArray.mkString + Set(5).toArray.mkString + "xy".toArray.mkString(",") + Map(1 -> "one").toArray.map(_._2).mkString)
  println(ints.filter(_ > 5).mkString(",") + " " + ints.reverse.mkString(",") + " " + ints.sorted.mkString(",") + " " + ints.take(2).mkString(",") + " " + ints.drop(2).mkString(",") + " " + ints.zip(strings).map((i, s) => s + i).mkString(","))
  println((ints ++ Array(1)).mkString(",") + " " + (ints :+ 0).mkString(",") + " " + (0 +: ints).mkString(",") + " " + ints.zipWithIndex.map(_._2).mkString + " " + strings.map(_.length).sum + " " + ints.updated(0, 1).mkString(","))
  val longer = Array.copyOf(ints, 2)
  val shorter = Array.copyOf(strings, 1)
  println(longer.mkString(",") + " " + shorter.mkString(","))
  val target = new Array[Int](4)
  System.arraycopy(ints, 0, target, 1, 3)
  java.util.Arrays.sort(target)
  println(target.mkString(","))
  val imm = IArray(1, 2, 3)
  println(imm.map(_ * 2).toList.toString + imm.length + imm(1) + IArray("a", "b").map(_.toUpperCase).mkString)
  val buffer = scala.collection.mutable.ArrayBuffer(1, 2)
  val from = buffer.toArray
  buffer += 3
  from(0) = 0
  println(from.mkString(",") + " " + buffer.mkString(","))
  println("a,b,,c".split(",").length.toString + "a b".split(" ").map(_.toUpperCase).mkString("-"))
  val doubles = Array(2.5, -1.5, 0.5)
  java.util.Arrays.sort(doubles)
  val words = Array("pear", "apple", "fig")
  java.util.Arrays.sort(words.asInstanceOf[Array[AnyRef]])
  val chars = "hzlao".toCharArray
  java.util.Arrays.sort(chars, 1, 4)
  java.util.Arrays.fill(target, 3)
  println(doubles.mkString(",") + " " + words.mkString(",") + " " + chars.mkString + " " + target.sum + " " + java.util.Arrays.equals(ints, ints.clone()) + " " + java.util.Arrays.toString(ints) + " " + java.util.Arrays.binarySearch(Array(1, 3, 5), 3))
