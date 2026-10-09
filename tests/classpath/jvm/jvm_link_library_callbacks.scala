// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// Members a library calls on a program's instance: an Ordering's compare, an anonymous
// Iterator's hasNext/next, an Ordered's compare. The reach pass keeps a program class's
// member only when program code calls its name, so in link mode these need to be kept.
final case class Money(cents: Long) extends Ordered[Money]:
  def compare(that: Money): Int = java.lang.Long.compare(cents, that.cents)
@main def run(): Unit =
  println(List(3, 1, 2).sorted(using Ordering.fromLessThan[Int](_ > _)))
  val it = new Iterator[Int]:
    var i = 0
    def hasNext = i < 3
    def next() = { i += 1; i }
  println(it.toList)
  println(List(Money(5), Money(2)).max)
