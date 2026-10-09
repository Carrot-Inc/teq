// The collection protocol scala-library's bodies reach on the std's collections: `coll`,
// `newSpecificBuilder`, `fromSpecific`, `className`, `copyToArray`, a builder's `sizeHint` over
// a collection, `Array.copyAs` and `copyOf`, and `ArrayBuilder`'s array overloads.
import scala.collection.mutable.{ArrayBuilder, Builder}

final class Bag[+A](private val items: Array[Any]) extends IndexedSeq[A]:
  def apply(i: Int): A = items(i).asInstanceOf[A]
  def length: Int = items.length
  override def className: String = "Bag"
  override def toString: String = mkString("Bag(", ",", ")")
  def protocol[B >: A](x: B, y: B): String = coll.toString + " " + fromSpecific(Iterator(x, y).map(_.asInstanceOf[A])) + " " + (newSpecificBuilder += x.asInstanceOf[A]).result() + " " + className

object Bag:
  def apply[A](elems: A*): Bag[A] = new Bag(elems.toArray[Any])

object Main:
  def main(args: Array[String]): Unit =
    val b = Bag(3, 1, 2)
    println(b)
    println(b.map(_ * 2))
    println(b.distinctBy(_ % 2))
    println(b.head + " " + b.last + " " + b.size)
    val dest = new Array[Int](5)
    println(b.copyToArray(dest, 1) + " " + dest.toList)
    println(b.copyToArray(dest, 3, 1) + " " + dest.toList)
    println(List(1, 2, 3).copyToArray(dest) + " " + dest.toList)
    println(Vector(9, 8).copyToArray(dest, 4) + " " + dest.toList)
    val builder: Builder[Int, List[Int]] = List.newBuilder[Int]
    builder.sizeHint(b)
    builder.sizeHint(b, 2)
    builder ++= b
    println(builder.result())
    println(b.protocol(4, 5))
    val copied = Array.copyAs[Int](Array(1, 2, 3), 5)
    println(copied.toList)
    println(Array.copyOf(Array("a", "b"), 1).toList)
    val ab = ArrayBuilder.make[String]
    ab.addAll(Array("p", "q"))
    ab.addAll(Array("r", "s", "t"), 1, 2)
    println(ab.result().toList)
