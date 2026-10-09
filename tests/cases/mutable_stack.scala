// scala-library's `mutable.Stack`, as zio's `Cause` and `ChannelExecutor` bodies use it: `push`,
// `pop`, `head`, `isEmpty`, `nonEmpty`, seen as an `Iterable`, printed.
import scala.collection.mutable

object Main:
  def main(args: Array[String]): Unit =
    val s = mutable.Stack[Int]()
    s.push(1)
    s.push(2).push(3)
    println(s)
    println(s.head)
    println(s.top)
    println(s.pop())
    println(s"${s.size} ${s.isEmpty} ${s.nonEmpty}")
    val it: Iterable[Int] = s
    println(it.toList)
    val t = new mutable.Stack[String]
    t.push("a")
    t.pushAll(List("b", "c"))
    println(t.mkString(","))
    val out = new StringBuilder
    while t.nonEmpty do out.append(t.pop())
    println(out)
    println(t.isEmpty)
    println(mutable.Stack(1, 2, 3).top)
    println(mutable.Stack(4, 5).map(_ * 2))
    val e = mutable.Stack.empty[Int]
    println(scala.util.Try(e.pop()).isFailure)
