// The three views keep scala-library's types: a sequence's view is a `SeqView`, an indexed
// sequence's an `IndexedSeqView`, and an operation keeps the kind of the view where
// scala-library's result type does (`map`, `take`, `reverse`), a `View` otherwise (`filter`).
import scala.collection.{View, SeqView, IndexedSeqView}

def kind(v: View[?]): String = v match
  case _: IndexedSeqView[?] => "IndexedSeqView"
  case _: SeqView[?] => "SeqView"
  case _ => "View"

@main def run(): Unit =
  val s: SeqView[Int] = List(1, 2, 3).view
  val i: IndexedSeqView[Int] = Vector(1, 2, 3).view
  val a: IndexedSeqView[Int] = Array(1, 2, 3).view
  val v: View[Int] = Set(1, 2).view
  val up: View[Int] = i
  val sv: SeqView[Int] = i
  println(List(kind(s), kind(i), kind(a), kind(v), kind(up), kind(sv)))
  println(List(s, i, a, v).map(_.toString))
  val sm: SeqView[Int] = s.map(_ * 10)
  val st: SeqView[Int] = s.take(2)
  val sd: SeqView[Int] = s.drop(1)
  val sr: SeqView[Int] = s.reverse
  val sa: SeqView[Int] = s.appended(4)
  val sp: SeqView[Int] = s.prepended(0)
  val sl: SeqView[Int] = s.sorted
  val sf: View[Int] = s.filter(_ > 1)
  val sfm: View[Int] = s.flatMap(x => List(x, x))
  println(List(sm, st, sd, sr, sa, sp, sl, sf, sfm).map(x => s"${kind(x)} ${x.toList}"))
  val im: IndexedSeqView[Int] = i.map(_ + 1)
  val it: IndexedSeqView[Int] = i.take(2)
  val ir: IndexedSeqView[Int] = i.reverse
  val is: IndexedSeqView[Int] = i.slice(1, 3)
  val ia: IndexedSeqView[Int] = i.appended(9)
  val if1: View[Int] = i.filter(_ != 2)
  println(List(im, it, ir, is, ia, if1).map(x => s"${kind(x)} ${x.toList}"))
  println(List(s.length, s(1), i.length, i(2), a(0)))
  println(s.map(_ + 1).filter(_ % 2 == 0).map(_.toString).mkString(","))
  println(i.reverse.map(_ * 2).take(2).toList)
  println(List(sm.toString, sf.toString, im.toString, if1.toString))
  val sc: SeqView[Int] = s.concat(List(4))
  val ic: IndexedSeqView[Int] = i.concat(Vector(4))
  val ip: IndexedSeqView[Int] = i.prependedAll(Vector(0))
  val s2: View[Int] = s ++ List(5)
  val ll: SeqView[Int] = LazyList(1, 2).view
  println(List(sc, ic, ip, s2, ll).map(x => s"${kind(x)} ${x.toList}"))
  println(View(1, 2).map(_ + 1).toList)
  val sai: View[Int] = s.appendedAll(Iterator(7))
  val sci: View[Int] = s.concat(Iterator(8))
  val sas: SeqView[Int] = s.appendedAll(List(7))
  val iai: IndexedSeqView[Int] = i.appendedAll(Array(7).toIndexedSeq)
  println(List(sai, sci, sas, iai).map(x => s"${kind(x)} ${x.toList}"))
  val ii: IndexedSeqView[Int] = i.concat(i.map(_ * 2))
  val ir2: IndexedSeqView[Int] = i.appendedAll(1 to 2)
  println(List(ii, ir2).map(x => s"${kind(x)} ${x.toList}"))
