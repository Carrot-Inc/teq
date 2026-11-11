// A sequence spread into a Java method's varargs is copied, an array spread is the array itself
// (`Stream.of(xs*)`, `IntStream.of`). And a collector's protocol as the JDK's `ReferencePipeline.collect` follows it: a
// parallel stream asks whether the collector is `CONCURRENT` and then, ordered, `UNORDERED`
// before the sequential stream's questions, whatever thread runs it. A `CONCURRENT` collector's one container
// takes the elements from several threads at once: the probe's is the JDK's concurrent queue, so that its
// count is the stream's two elements however the threads run (`synchronized` in the program would not do: it
// takes no monitor in teq's output, docs/COMPATIBILITY.md).
import java.util.stream.{Collector, IntStream, Stream}
import java.util.function.{Supplier, BiConsumer, BinaryOperator, Function}
import java.util.concurrent.ConcurrentLinkedQueue
import scala.collection.immutable.ArraySeq

class Probe(concurrent: Boolean, unordered: Boolean) extends Collector[String, ConcurrentLinkedQueue[String], Int]:
  def supplier(): Supplier[ConcurrentLinkedQueue[String]] = { println("supplier"); () => new ConcurrentLinkedQueue[String]() }
  def accumulator(): BiConsumer[ConcurrentLinkedQueue[String], String] = { println("accumulator"); (a, s) => { a.add(s); () } }
  def combiner(): BinaryOperator[ConcurrentLinkedQueue[String]] = { println("combiner"); (a, b) => { a.addAll(b); a } }
  def finisher(): Function[ConcurrentLinkedQueue[String], Int] = { println("finisher"); a => a.size() }
  def characteristics(): java.util.Set[Collector.Characteristics] = new java.util.HashSet[Collector.Characteristics]():
    override def contains(x: Any): Boolean =
      println("asked " + x)
      (x == Collector.Characteristics.CONCURRENT && concurrent) || (x == Collector.Characteristics.UNORDERED && unordered)

object Main:
  def main(args: Array[String]): Unit =
    val a = Array("before")
    val fromSeq = Stream.of(ArraySeq.unsafeWrapArray(a)*)
    val fromArray = Stream.of(a*)
    a(0) = "after"
    println(fromSeq.findFirst().get())
    println(fromArray.findFirst().get())
    val b = Array(1)
    val ints = IntStream.of(ArraySeq.unsafeWrapArray(b)*)
    b(0) = 2
    println(ints.sum())
    println(java.util.List.of(List("x", "y")*))
    println(Stream.of("x", "y").collect(new Probe(false, false)))
    println(Stream.of("x", "y").parallel().collect(new Probe(false, false)))
    println(Stream.of("x", "y").parallel().collect(new Probe(true, false)))
    println(Stream.of("x", "y").parallel().collect(new Probe(true, true)))
    println(Stream.of("x", "y").parallel().unordered().collect(new Probe(true, false)))
