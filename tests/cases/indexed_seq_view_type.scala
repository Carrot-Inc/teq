// The views of sequences by the names of scala-library's types, `IndexedSeqView` and `SeqView`:
// what a library's signatures say of `view` on an indexed sequence, a string and a list.
import scala.collection.{IndexedSeqView, SeqView}

@main def run =
  val v: IndexedSeqView[Int] = Vector(1, 2, 3, 4).view
  println(v.slice(1, 3).toList)
  println(v.take(2).concat(v.drop(3)).toList)
  println(v.size)
  println(v(2))
  println(v.isEmpty)
  val s: IndexedSeqView[Char] = "hello".view
  println(s.slice(1, 4).mkString)
  println(s.toIndexedSeq)
  val q: SeqView[Int] = List(1, 2).view
  println(q.map(_ * 2).toList)
