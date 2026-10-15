// An overloaded method given where a function type is expected that no alternative fits as a whole (indigo's
// `events.foreach(systemActions.enqueue)`): the alternative that applies to the function's parameter types, its
// `this.type` result discarded for the `Unit` one, as dotty's `resolveOverloaded` narrows by types (`narrowByTypes`,
// `constrainResult` disregarding a `Unit` result).
import scala.collection.mutable

final class Batch[A](items: List[A]):
  def foreach(f: A => Unit): Unit = items.foreach(f)

class Sink:
  def put(x: Int): this.type = this
  def put(x: Int, y: Int, more: Int*): this.type = this

@main def run(): Unit =
  val q = new mutable.Queue[Int]()
  Batch(List(1, 2, 3)).foreach(q.enqueue)
  println(q.toList)
  val s = new Sink
  Batch(List(4)).foreach(s.put)
  println("ok")
