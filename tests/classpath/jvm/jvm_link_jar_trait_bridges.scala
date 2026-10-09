// jars: scala-library
// std: scala-library
// Classes of the program that extend scala-library's traits directly get the bridges scalac's
// mixin phase adds, which the jar's bytecode calls: `PartialOrdering.tryCompare` returning
// `Option` beside `Ordering`'s default returning `Some`, `IterableOnceOps.filter` and `map`
// returning `Object` beside `Iterator`'s defaults.
final case class Money(cents: Long)
object MoneyOrdering extends Ordering[Money]:
  def compare(a: Money, b: Money): Int = java.lang.Long.compare(a.cents, b.cents)
def partial[A](o: PartialOrdering[A], x: A, y: A): Option[Int] = o.tryCompare(x, y)
def filtered[A](xs: scala.collection.IterableOnceOps[A, Iterator, Iterator[A]], p: A => Boolean): Iterator[A] = xs.filter(p)
@main def run(): Unit =
  println(partial(MoneyOrdering, Money(1), Money(2)))
  val byLength = new Ordering[String]:
    def compare(a: String, b: String): Int = a.length - b.length
  println(partial(byLength, "aaa", "b"))
  println(List("ccc", "a", "bb").sorted(using byLength))
  val it = new Iterator[Int]:
    var i = 0
    def hasNext = i < 5
    def next() = { i += 1; i }
  println(filtered(it, _ % 2 == 1).toList)
  val it2 = new Iterator[Int]:
    var i = 0
    def hasNext = i < 3
    def next() = { i += 1; i }
  println(it2.map(_ * 10).toList)
