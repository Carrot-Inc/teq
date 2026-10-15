// The std's `java.lang.Math` trigonometry, `mutable.Queue`, `scala.util.control.Breaks` and `Array.flatten`,
// as indigo calls them (`import util.control.Breaks.*`, a lean build's class path without scala-library).
import scala.collection.mutable
import scala.util.control.Breaks.*

@main def run(): Unit =
  println(s"${Math.round(Math.acos(0.5) * 1000)} ${Math.round(Math.toDegrees(Math.PI))} ${Math.round(Math.asin(1.0) * 1000)}")
  val q = new mutable.Queue[Int]()
  q.enqueue(1); q.enqueue(2); q += 3
  println(s"${q.dequeue()} ${q.size} ${q.toList}")
  q.enqueue(4)
  println(s"${q.dequeueAll(_ % 2 == 0).toList} ${q.toList} ${q.front}")
  var n = 0
  breakable {
    for i <- 1 to 10 do
      if i > 3 then break()
      n += i
  }
  println(n)
  println(Array(Array(1, 2), Array(3)).flatten.toList)
  println(tryBreakable { break(); 1 } catchBreak { 2 })
  println(tryBreakable { 3 } catchBreak { 4 })
