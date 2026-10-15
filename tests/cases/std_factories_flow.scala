// scala-library's collection companions are IterableFactories (fs2's `to(List)`, cats' `Chain`
// passing `Vector`), with the generic `range` of `Integral`; a TreeMap mapped to pairs is a TreeMap
// (cats' `NonEmptyVector.groupBy`), an iterator compares its elements with another's, and the
// JDK's reactive-streams interfaces are there to implement (fs2's `interop.flow`).
import scala.collection.immutable.{Queue, TreeMap}
import java.util.concurrent.Flow

def make[C[_]](f: scala.collection.IterableFactory[C]): C[Int] = f.from(List(3, 1, 2))

final class Collect extends Flow.Subscriber[Int]:
  var seen: List[Int] = Nil
  def onSubscribe(s: Flow.Subscription): Unit = s.request(2)
  def onNext(item: Int): Unit = seen = item :: seen
  def onError(t: Throwable): Unit = ()
  def onComplete(): Unit = println("complete " + seen.reverse)

@main def run(): Unit =
  println(make(List))
  println(make(Vector))
  println(make(Queue))
  println(List.range(0, 3) ++ List.range(0, 6, 2) ++ Vector.range(1L, 3L) ++ List.range(5, 0, -2))
  println(List.concat(List(1), Vector(2)))
  val t = TreeMap(2 -> "b", 1 -> "a")
  val u: TreeMap[Int, String] = t.map { case (k, v) => (k * 10, v * 2) }
  println(u)
  println(Queue.newBuilder[Int].addOne(1).result())
  println(s"${Iterator(1, 2).sameElements(List(1, 2))} ${Iterator(1, 2).sameElements(List(1))}")
  val c = new Collect
  c.onSubscribe(new Flow.Subscription:
    def request(n: Long): Unit = println("request " + n)
    def cancel(): Unit = ())
  c.onNext(4)
  c.onNext(5)
  c.onComplete()
