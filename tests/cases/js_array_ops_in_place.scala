//> using platform js
// Scala.js's in-place operations of `js.ArrayOps` (`+=:`, `+=`, `insert`, `remove`, `filterInPlace`),
// `toJSDictionary` of a mutable map and `toJSPromise` of a future, as indigo calls them.
import scala.scalajs.js
import scala.scalajs.js.JSConverters.*
import scala.collection.mutable
import scala.concurrent.Future
import scala.concurrent.ExecutionContext.Implicits.global

@main def run(): Unit =
  val a = js.Array(2, 3)
  1 +=: a
  a += 4
  a ++= List(5, 6)
  a.insert(0, 0)
  println(a.toList)
  println(s"${a.remove(1)} ${a.toList}")
  a -= 5
  a.filterInPlace(_ % 2 == 0)
  println(a.toList)
  val m = mutable.HashMap("k" -> 1)
  println(m.toJSDictionary("k"))
  Future(41).toJSPromise.`then`[Unit](v => println(v + 1))
