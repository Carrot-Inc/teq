// `scala.collection.Seq` and `scala.collection.IndexedSeq` above the std's `Seq` and `IndexedSeq` (scala-library's
// immutable ones): an immutable sequence is a `collection.Seq`, whose operations, equality, hash, extractors,
// ordering and companion work as scala-library's.
import scala.collection

def total(xs: collection.Seq[Int]): Int = xs.sum
def firstTwo(xs: collection.Seq[Int]): String = xs match
  case a +: b +: _ => s"$a,$b"
  case _ => "short"
def lastOf(xs: collection.IndexedSeq[Int]): Int = xs match
  case _ :+ last => last
  case _ => -1

@main def main(): Unit =
  val list: Seq[Int] = List(1, 2, 3)
  val general: collection.Seq[Int] = list
  println(total(general) + total(Vector(4, 5)))
  println(general.map(_ * 2))
  println(firstTwo(general) + " " + firstTwo(List(9)))
  println(lastOf(Vector(7, 8)) + " " + lastOf(IndexedSeq(1)))
  println(general == List(1, 2, 3))
  println(general.hashCode == List(1, 2, 3).hashCode)
  val made = collection.Seq(3, 1, 2)
  println(made.sorted)
  println(List(collection.Seq(2, 1), collection.Seq(1, 5)).sorted(using Ordering.Implicits.seqOrdering[collection.Seq, Int]))
  println(collection.IndexedSeq(1, 2).length)
