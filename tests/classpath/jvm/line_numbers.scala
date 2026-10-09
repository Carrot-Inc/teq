// jars: scala-library
// std: scala-library
// Class files carry a line table with the line of each expression's point, a selection's at its member name, so a stack trace names the line of the throw and of each call on the way, as scalac's does.
object Main:
  def lines(e: Throwable): String =
    e.getStackTrace.filter(_.getClassName.startsWith("Main$")).map(_.getLineNumber).mkString(",")

  def boom(n: Int): Int =
    val a = n + 1
    val b = a * 2
    if b > 3 then
      throw new RuntimeException("boom " + b)
    b

  def chain(xs: List[Int]): Int =
    xs
      .map(x =>
        if x > 1 then throw new RuntimeException("chain")
        else x)
      .filter(_ > 0)
      .sum

  def three(a: Int, b: Int, c: Int): Int = a + b + c
  def spread(): Int =
    three(
      1,
      boom(1),
      3)

  class Thing(s: String):
    if s.isEmpty then
      throw new IllegalArgumentException("empty")
    val size = s.length

  def made(): Int =
    new Thing(
      "").size

  def matched(n: Int): Int = n match
    case 1 =>
      10
    case _ =>
      throw new RuntimeException("matched")

  def comprehension(): Option[Int] =
    for
      a <- Some("a")
      b <- Some("b")
    yield
      if (a + b).length > 1 then throw new RuntimeException("yield")
      else a.length

  def looped(n: Int): Int =
    var i = 0
    while i < n do
      i += 1
      if i == 3 then
        throw new RuntimeException("loop")
    i

  def sorted(xs: List[Int]): List[Int] =
    xs
      .map(x => x)
      .sortBy(x =>
        if x > 0 then throw new RuntimeException("sort") else x)

  class Trace
  given Trace = new Trace
  def step(n: Int)(using Trace): Int =
    if n > 2 then throw new RuntimeException("step") else n
  def pipeline(): Int =
    step(1)
      + step(2)
      + step(3)

  def catching(f: () => Any): String =
    try
      f()
      "none"
    catch case e: Exception => lines(e)

  def main(args: Array[String]): Unit =
    println(catching(() => boom(1)))
    println(catching(() => chain(List(1, 2))))
    println(catching(() => spread()))
    println(catching(() => made()))
    println(catching(() => matched(2)))
    println(catching(() => comprehension()))
    println(catching(() => looped(5)))
    println(catching(() => sorted(List(2, 1))))
    println(catching(() => pipeline()))
