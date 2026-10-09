// `scala.collection.BuildFrom` as a generic traversal takes it to rebuild the collection it was
// given: the collection's own kind, a map, a sorted set, a string, an array.
import scala.collection.BuildFrom
import scala.collection.immutable.SortedSet

object Main:
  def traverse[CC[+X] <: Iterable[X], A, B](in: CC[A])(f: A => B)(using bf: BuildFrom[CC[A], B, CC[B]]): CC[B] =
    val b = bf.newBuilder(in)
    in.foreach(a => b += f(a))
    b.result()

  def main(args: Array[String]): Unit =
    println(traverse(List(1, 2, 3))(_ * 2))
    println(traverse(Vector("a", "b"))(_.toUpperCase))
    println(traverse(1 to 3)(_.toString))
    println(traverse(Set(1, 2))(_ + 1))
    val seq: Seq[Int] = List(4, 5)
    println(traverse(seq)(_ - 1))
    println(summon[BuildFrom[Map[Int, String], (String, Int), Map[String, Int]]].fromSpecific(Map(1 -> "a"))(List("b" -> 2)))
    println(summon[BuildFrom[SortedSet[Int], Int, SortedSet[Int]]].fromSpecific(SortedSet(3))(List(9, 1, 5)))
    println(summon[BuildFrom[String, Char, String]].fromSpecific("ab")("xyz".iterator))
    println(summon[BuildFrom[Array[Int], Int, Array[Int]]].fromSpecific(Array(1))(List(7, 8)).toList)
    println(summon[BuildFrom[List[Int], String, List[String]]].toFactory(List(1)).fromSpecific(List("q")))
