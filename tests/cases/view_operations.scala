// The views' operations as scala-library runs them: a sequence's view indexes its source, so an
// element of a mapped view applies the function to that element alone and a length applies it to
// none; each operation is a lazy adapter over its source, a sorted view sorts once and keeps the
// result, tails and inits are views taken on demand; a program's IndexedSeqView implements length
// and apply alone.
import scala.collection.{View, SeqView, IndexedSeqView}

var calls = 0
def f(x: Int): Int = { calls += 1; x * 10 }
def counted(label: String)(body: => Any): Unit =
  calls = 0
  val r = body
  println(s"$label: $r, $calls calls")

class Squares(n: Int) extends IndexedSeqView[Int]:
  def length: Int = n
  def apply(i: Int): Int = i * i

@main def run(): Unit =
  val s: SeqView[Int] = List(1, 2, 3).view.map(f)
  counted("apply")(s(2))
  counted("length")(s.length)
  counted("toList")(s.toList)
  counted("reverse apply")(s.reverse(0))
  counted("take length")(s.take(2).length)
  counted("drop apply")(s.drop(1)(0))
  counted("takeRight apply")(s.takeRight(1)(0))
  counted("dropRight length")(s.dropRight(1).length)
  counted("appended apply")(s.appended(7)(3))
  counted("prepended apply")(s.prepended(7)(1))
  counted("concat apply")(s.concat(List(4, 5))(4))
  counted("sorted twice") { val t = s.sorted; (t.toList, t.toList, t(1)) }
  counted("sorted reverse")(s.sorted(using Ordering.Int.reverse).toList)
  counted("tails")(s.tails.map(_.size).toList)
  counted("inits")(s.inits.map(_.size).toList)
  counted("isEmpty")(s.isEmpty)
  val i: IndexedSeqView[Int] = Vector(1, 2, 3).view.map(f)
  counted("indexed apply")(i(1))
  counted("indexed slice apply")(i.slice(1, 3)(0))
  counted("indexed iterator")(i.iterator.next())
  counted("indexed reverse")(i.reverse.toList)
  val q = new Squares(4)
  println(q.toList)
  println(q.map(_ + 1).toList)
  println(q.reverse.take(2).toList)
  println((q(3), q.length, q.slice(1, 3).toList, q.sum))
  println(q.filter(_ % 2 == 0).toList)
