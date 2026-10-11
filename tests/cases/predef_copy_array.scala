// scala-library's `Predef.copyArrayToImmutableIndexedSeq` called as it is written: an immutable
// `IndexedSeq` copied from the array, which the array's later writes leave alone; `null` for a
// `null` array. The lean std has it as a method alone (`std/collections.scala`).
import scala.annotation.nowarn

@nowarn("cat=deprecation")
def copied[T](a: Array[T]): IndexedSeq[T] = Predef.copyArrayToImmutableIndexedSeq(a)

@main def run(): Unit =
  val a = Array(1, 2, 3)
  val c = copied(a)
  a(0) = 9
  println(c.mkString(",") + " " + a.mkString(","))
  println(c.length + c.sum)
  println(copied(Array("x", "y")).reverse.mkString)
  val n: Array[Int] = null
  println(copied(n) == null)
