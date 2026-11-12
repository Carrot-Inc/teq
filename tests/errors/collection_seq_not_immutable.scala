// The lean std is to keep scala-library's sequence hierarchy where a program names it: `scala.collection.Seq` and
// `scala.collection.IndexedSeq` are to be the std's own traits above `Seq` (scala-library's `immutable.Seq`) and
// `IndexedSeq`, so that a `collection.Seq` takes no immutable sequence's place and spreads into no repeated
// parameter, which wants an `immutable.Seq`. The expected messages name the two by their qualified names.
def immutable(xs: scala.collection.Seq[Int]): scala.collection.immutable.Seq[Int] = xs
def count(xs: Int*): Int = xs.size
def spread(xs: scala.collection.Seq[Int]): Int = count(xs*)
def indexed(xs: scala.collection.IndexedSeq[Int]): IndexedSeq[Int] = xs
// expect: 5:85: error: type mismatch: found scala.collection.Seq[Int], required scala.Seq[Int]
// expect: 7:56: error: type mismatch: found scala.collection.Seq[Int], required scala.Seq[Int] | Array[? <: Int]
// expect: 8:70: error: type mismatch: found scala.collection.IndexedSeq[Int], required scala.IndexedSeq[Int]
