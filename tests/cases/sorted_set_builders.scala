// scala-library's sorted-set builders, branch by branch. The immutable `TreeSetBuilder` (over `SetHelper.mutableUpd`):
// insertions compared as its red-black tree compares them (the comparisons logged), an element added again by
// reference not compared, `++=` of a set of the same ordering adopting its tree into an empty builder (no comparison)
// and uniting with it otherwise, the adopted set unchanged by what the builder adds after, a result leaving the builder
// as it was; `flatMap` adding each function result through `++=`. The mutable `TreeSet`'s builder: each result a new
// set over the one tree the builder and its results share, `clear` giving the builder a new tree.
import scala.collection.immutable.TreeSet
import scala.collection.mutable

object Main:
  def attempt(r: => Any): Unit =
    try println(r)
    catch case e: Exception => println(e.getMessage)
  def main(args: Array[String]): Unit =
    val log = mutable.ArrayBuffer.empty[String]
    val logged: Ordering[Int] = new Ordering[Int]:
      def compare(x: Int, y: Int): Int =
        log += s"$x?$y"
        Integer.compare(x, y)
    val b = TreeSet.newBuilder[Int](using logged)
    b ++= List(5, 1, 4, 2, 3, 4)
    println(s"${b.result()} ${log.mkString(" ")}")
    log.clear()
    println(s"${TreeSet(1, 2, 3)(using Ordering.Int).map(x => 4 - x)(using logged)} ${log.mkString(" ")}")
    log.clear()
    val rl = TreeSet.newBuilder[Int](using logged)
    rl ++= List(1, 3, 2, 6, 5, 4)
    println(s"${rl.result()} ${log.mkString(" ")}")

    // The same element by reference is no comparison.
    class Token
    val token = new Token
    val bad: Ordering[Token] = new Ordering[Token]:
      def compare(x: Token, y: Token): Int = throw new IllegalStateException("comparison")
    attempt(TreeSet(1, 2).map(_ => token)(using bad).size)

    // A set of the same ordering adopted by an empty builder, compared with nothing; the adopted set unchanged after.
    var allowed = true
    val strict: Ordering[Int] = new Ordering[Int]:
      def compare(x: Int, y: Int): Int =
        if !allowed then throw new IllegalStateException("comparison")
        Integer.compare(x, y)
    val src = TreeSet(1, 2, 3)(using strict)
    allowed = false
    val adopting = TreeSet.newBuilder[Int](using strict)
    attempt { adopting ++= src; adopting.result().toList }
    attempt(TreeSet(0).flatMap(_ => src)(using strict).toList)
    allowed = true
    adopting += 0
    val again = TreeSet.newBuilder[Int](using strict)
    again ++= src
    println(s"${adopting.result()} $src ${again.result()}")
    // A set added twice: the second union is the tree itself (`t1 eq t2`), no comparison; the adopted tree read again.
    allowed = false
    val twice = TreeSet.newBuilder[Int](using strict)
    attempt { twice ++= src; twice ++= src; twice.result().toList }
    val reread = TreeSet.newBuilder[Int](using strict)
    attempt { reread ++= again.result(); reread.result().toList }
    allowed = true
    // A nonempty builder unites with the set.
    val uniting = TreeSet.newBuilder[Int](using strict)
    uniting += 9
    uniting += 2
    uniting ++= src
    println(uniting.result())
    // A map of the same ordering, adopted as a set of its keys (a branch typed code reaches only through a cast).
    val asKeys = scala.collection.immutable.TreeMap(1 -> "a", 2 -> "b")(using strict).asInstanceOf[IterableOnce[Int]]
    allowed = false
    val keys = TreeSet.newBuilder[Int](using strict)
    attempt { keys ++= asKeys; keys.result().toList }
    allowed = true
    // A result leaves the builder as it was.
    val reused = TreeSet.newBuilder[Int]
    reused += 5
    val first = reused.result()
    reused += 4
    println(s"$first ${reused.result()}")
    reused.clear()
    reused += 7
    println(reused.result())

    // The mutable set's builder: results over one shared tree, `clear` a new one.
    val mb = mutable.TreeSet.newBuilder[Int]
    mb += 1
    val x = mb.result()
    val y = mb.result()
    println(x eq y)
    x += 2
    println(y.toList)
    mb.clear()
    mb += 3
    println(s"${x.toList} ${mb.result().toList}")
    // Removals through one of the sets sharing the tree are the other's too; `filterInPlace` sees the set shrink.
    val mb2 = mutable.TreeSet.newBuilder[Int]
    mb2 ++= List(1, 2, 3, 4, 5)
    val u = mb2.result()
    val v = mb2.result()
    u -= 1
    u --= List(2)
    u.filterInPlace { e => print(s"p($e) size ${v.size} "); e != 4 }
    println(v.toList)
    // The shared tree cleared through one of its sets is cleared for the other.
    x.clear()
    println(s"${y.toList} ${x.toList}")
    // A builder mapped (`mapResult`) adopts as the builder does.
    allowed = false
    val mapped = TreeSet.newBuilder[Int](using strict).mapResult(_.toList)
    attempt { mapped ++= src; mapped.result() }
    allowed = true
