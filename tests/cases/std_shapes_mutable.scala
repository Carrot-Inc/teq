// The shapes of scala-library that library bodies build the mutable collections with: the
// no-argument constructors, `ArrayBuffer(initialSize)`, `getOrElse` widening the value type,
// `lastIndexWhere(p, end)` and `indexWhere(p, from)` on a `String`.
package stdshapesmutable

import scala.collection.mutable

object Main:
  def main(args: Array[String]): Unit =
    val m = new mutable.LinkedHashMap[String, List[String]]()
    m.update("a", m.getOrElse("a", Nil) ++ List("x"))
    m.update("a", m.getOrElse("a", Nil) ++ List("y"))
    val any: Any = m.getOrElse("b", "none")
    println(m("a").toString + " " + m.size + " " + any)
    val hm = new mutable.HashMap[Int, Int]()
    hm(1) = 2
    val hs = new mutable.HashSet[Int]()
    hs += 3
    println(hm(1).toString + " " + hs.contains(3) + " " + hs.size)
    val buf = new mutable.ArrayBuffer[Int](32)
    buf += 1
    buf += 2
    buf += 3
    buf += 2
    println(buf.lastIndexWhere(_ == 2).toString + " " + buf.lastIndexWhere(_ == 2, 2) + " " + buf.lastIndexWhere(_ == 9, 1) + " " + new mutable.ArrayBuffer[String]().length + " " + List(1, 2, 1).lastIndexWhere(_ == 1, 1))
    println("hello world".indexWhere(_ == 'o').toString + " " + "hello world".indexWhere(_ == 'o', 5) + " " + "hello".indexWhere(_ == 'z', 1))
