// scala.collection.immutable.Queue: empty, enqueue, :+, dequeue, iteration and the
// collection operations izumi-reflect's inspector keeps its lambda contexts in.
import scala.collection.immutable.Queue

object Main:
  def main(args: Array[String]): Unit =
    val q = Queue.empty[Int] :+ 1 :+ 2
    val q2 = q.enqueue(3).enqueueAll(List(4, 5))
    println(q2)
    val (h, rest) = q2.dequeue
    println(s"$h $rest ${rest.size} ${rest.last} ${rest.reverse}")
    println(q2.flatMap(i => List(i, i * 10)).toSet.size)
    println(Queue(1, 2, 3).map(_ + 1).filter(_ > 2))
    println(Queue.empty[String].dequeueOption)
    println(Queue("a") == Queue("a"))
