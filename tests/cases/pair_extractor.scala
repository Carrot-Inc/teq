// `case k -> v` takes a pair apart through scala-library's `->` extractor, in a match, a
// pattern val and a for comprehension.
object Main:
  def main(args: Array[String]): Unit =
    val m = Map(1 -> "a", 2 -> "b")
    m.toList.foreach { case k -> v => println(s"$k=$v") }
    val (x -> y) = (3, "c")
    println(x + y)
    val pairs = List("p" -> 1, "q" -> 2)
    for (name -> n) <- pairs do println(name * n)
    val nested = List((1, (2, 3)))
    nested.foreach { case a -> (b -> c) => println(a + b + c) }
    println(pairs.collect { case name -> n if n > 1 => name })
