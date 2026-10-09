// jars: scala-library
// Checks with no error: what a program gets from scala-library alone.
case class Point(x: Int, y: Int)

given Ordering[Point] with
  def compare(a: Point, b: Point): Int = if a.x != b.x then a.x - b.x else a.y - b.y

object Main:
  def total(xs: List[Int]): Int = xs.foldLeft(0)((acc, x) => acc + x)

  def sum(xs: List[Int]): Int = xs match
    case h :: t => h + sum(t)
    case Nil => 0

  def show(o: Option[Int]): String = o match
    case Some(x) => s"some $x"
    case None => "none"

  def either(e: Either[String, Int]): Int = e match
    case Left(msg) => msg.length
    case Right(n) => n

  def swap(p: (Int, String)): (String, Int) = p match
    case (a, b) => (b, a)

  def main(args: Array[String]): Unit =
    val xs = List(1, 2, 3)
    val ys = xs.map(_ + 1).filter(_ > 2)
    println(total(ys))
    println(sum(xs))
    val sorted: List[Int] = xs.sorted
    println(sorted)
    val o: Option[Int] = Some(4)
    val doubled: Int = o.map(_ * 2).getOrElse(0)
    println(doubled)
    println(o.fold("none")(n => s"some $n"))
    println(show(None))
    val m: Map[Int, String] = Map((1, "one"), (2, "two"))
    val found: Option[String] = m.get(1)
    println(found)
    val v: Vector[Int] = Vector(3, 1, 2)
    println(v.sorted)
    println(math.max(1, 2))
    println(math.abs(-3.5))
    val pairs: List[(Int, Int)] = for
      x <- xs
      y <- ys
      if x < y
    yield (x, y)
    println(pairs)
    val points = List(Point(2, 1), Point(1, 5), Point(1, 2))
    println(points.sorted)
    val ord: Ordering[Int] = implicitly[Ordering[Int]]
    println(ord.compare(1, 2))
    println(either(Left("abc")) + either(Right(2)))
    println(swap((1, "a")))
    println(xs.headOption)
    println(xs.zipWithIndex.map((x, i) => x * i).sum)
    println(Int.MaxValue)
    println("abc".toUpperCase.length)
    val caught: String =
      try List.empty[Int].head.toString
      catch
        case e: NoSuchElementException => "empty: " + e.getMessage
        case scala.util.control.NonFatal(e) => "other: " + e.toString
    println(caught)
    val ref: AnyRef = xs
    println(ref.toString.length > 0 && ref != null)
