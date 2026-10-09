// Collections scala-library has and teq's lean std lacks: TreeMap, SortedSet, LazyList.
import scala.collection.immutable.{TreeMap, SortedSet, LazyList}

object Main {
  def main(args: Array[String]): Unit = {
    val words = List("pear", "apple", "fig", "plum", "kiwi", "apple", "fig")
    val counts = words.foldLeft(TreeMap.empty[String, Int])((m, w) => m.updated(w, m.getOrElse(w, 0) + 1))
    println(counts)
    println(counts.range("b", "l").keys.mkString(","))
    val lengths = SortedSet(words.map(_.length): _*)
    println(lengths.toString + " " + lengths.min + " " + lengths.max)
    val fib: LazyList[Int] = 0 #:: 1 #:: fib.zip(fib.tail).map((a, b) => a + b)
    println(fib.take(12).toList)
    println(LazyList.from(1).filter(_ % 7 == 0).map(_ * 2).take(5).mkString(" "))
  }
}
