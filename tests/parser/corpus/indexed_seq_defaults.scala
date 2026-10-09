// A user IndexedSeq that defines only `apply` and `length` iterates through the defaults.
package indexedseqdefaults

final class Evens(n: Int) extends IndexedSeq[Int]:
  def apply(i: Int): Int = i * 2
  def length: Int = n

@main def main(): Unit =
  val e = Evens(4)
  e.foreach(x => print(s"$x "))
  println()
  val it = e.iterator
  println(it.next())
  println(it.hasNext)
  println(e.toList)
  println(e.mkString(","))
  println(e.sum)
  println(e.map(_ + 1).toList)
  println(e.contains(6))
  println(e.last)
  println(e.reverse.toList)
