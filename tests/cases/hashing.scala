// hashCode follows scala.util.hashing.MurmurHash3: products, sequences (with the range special
// case), unordered collections, and the name hash of parameterless cases.
case class P(x: Int, y: String)
case class Q(x: Int, y: String)
case class W[A](a: A)
case object Single
enum Color:
  case Red, Green
enum Shape:
  case Circle(r: Int)
  case Sq()
sealed trait T
case class TA(n: Int) extends T

@main def main(): Unit =
  println(s"${"".hashCode} ${"a".hashCode} ${"hello".hashCode} ${"Hello, World!".hashCode}")
  println(s"${1.hashCode} ${(-1).hashCode} ${1L.hashCode} ${(1L << 40).hashCode} ${(-1L).hashCode} ${123456789012345678L.hashCode}")
  println(s"${true.hashCode} ${false.hashCode} ${'a'.hashCode} ${().hashCode}")
  println(s"${1.##} ${(-1L).##} ${"ab".##} ${(1L << 40).##}")
  println(s"${(1, 2).hashCode} ${(1, "a", true).hashCode} ${(1, "a", true, 4L).hashCode} ${((1, 2), 3).hashCode}")
  println(s"${P(1, "a").hashCode} ${Q(1, "a").hashCode} ${W(1).hashCode} ${W(1L).hashCode} ${W("a").hashCode}")
  println(s"${Single.hashCode} ${TA(7).hashCode} ${Shape.Circle(1).hashCode} ${Shape.Sq().hashCode}")
  println(s"${Color.Red.hashCode} ${Color.Green.hashCode} ${(Color.Red.hashCode == Color.Red.hashCode)}")
  println(s"${List(1, 2, 3).hashCode} ${Vector(1, 2, 3).hashCode} ${(1 to 3).hashCode} ${List(1, 3, 6).hashCode} ${List(1).hashCode} ${List(2, 2).hashCode}")
  println(s"${Nil.hashCode} ${List().hashCode} ${List("a", "b").hashCode} ${List(List(1), List(2)).hashCode}")
  println(s"${Set(1, 2, 3).hashCode} ${Set(3, 2, 1).hashCode} ${Set[Int]().hashCode} ${Set("a").hashCode}")
  println(s"${Map(1 -> "a", 2 -> "b").hashCode} ${Map(2 -> "b", 1 -> "a").hashCode} ${Map[Int, Int]().hashCode}")
  println(s"${Some(1).hashCode} ${None.hashCode} ${Right(1).hashCode} ${Left("a").hashCode} ${Option(1).hashCode}")
  println(s"${scala.collection.mutable.ArrayBuffer(1, 2, 4).hashCode} ${scala.collection.mutable.HashSet(1, 2).hashCode} ${scala.collection.mutable.HashMap(1 -> 2).hashCode}")
  println(s"${(List(1.0, 2.0).hashCode == List(1, 2).hashCode)} ${(Set(1).hashCode == Set(1L).hashCode)} ${(W(1) .hashCode == W(1L).hashCode)}")
  val pairs = for x <- 0 until 300; y <- 0 until 300 yield P(x, "").hashCode
  println(pairs.distinct.size >= 89000)
