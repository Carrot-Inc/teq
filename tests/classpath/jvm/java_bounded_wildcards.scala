// jars: scala-library
// std: scala-library
// Java's bounded wildcards read as wildcards (`String.join(",", xs.asJava)` for a
// `Iterable<? extends CharSequence>`, jsprit's `Collection<? extends Vehicle>`), with lambdas for
// `Function<? super T, ? extends R>` implementing the SAM at the bound each parameter's variance
// in the method picks, as scalac's `samParent` does.
import scala.jdk.CollectionConverters.*

object Main:
  def main(args: Array[String]): Unit =
    val l = new java.util.ArrayList[String]()
    l.add("x"); l.add("yy")
    val s = java.util.stream.Stream.of("a", "bb").map(x => x.length).toList()
    println(s)
    l.forEach(x => println(x))
    val o = java.util.Optional.of("abc").map(x => x.length)
    println(o.get)
    val it = java.util.stream.Stream.of("a", "bb", "").filter(_.nonEmpty).iterator()
    while it.hasNext do println(it.next().length)
    println(String.join(",", List("a", "b").asJava))
    val m = new java.util.TreeMap[String, Integer]()
    m.put("k", 1)
    m.merge("k", 2, (a, b) => Integer.valueOf(a + b))
    println(m)
    val cs: java.util.Collection[? <: CharSequence] = List("q").asJava
    println(cs.iterator().next().length)
