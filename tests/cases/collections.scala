//> using platform js
case class Employee(name: String, dept: String, salary: Int)

def safeDiv(a: Int, b: Int): Either[String, Int] =
  if b == 0 then Left("division by zero") else Right(a / b)

def parseAge(s: String): Option[Int] =
  s.toIntOption.filter(n => n >= 0 && n < 150)

@main def run(): Unit =
  val xs = List(5, 3, 8, 1, 9, 2)
  println(xs.map(_ * 2))
  println(xs.filter(_ % 2 == 1))
  println(xs.foldLeft(0)(_ + _))
  println(xs.foldRight(List.empty[Int])((x, acc) => x :: acc))
  println(xs.reduce((a, b) => if a > b then a else b))
  println(xs.sorted)
  println(xs.sortWith(_ > _))
  println(xs.take(2))
  println(xs.drop(4))
  println(xs.takeWhile(_ > 2))
  println(xs.dropWhile(_ > 2))
  println(xs.reverse)
  println(xs.head)
  println(xs.tail)
  println(xs.last)
  println(xs.length)
  println(xs.isEmpty)
  println(xs.contains(8))
  println(xs.indexOf(8))
  println(xs.find(_ > 5))
  println(xs.find(_ > 50))
  println(xs.exists(_ == 1))
  println(xs.forall(_ > 0))
  println(xs.count(_ > 2))
  println(xs.zip(List("a", "b", "c")))
  println(xs.zipWithIndex.take(2))
  println(xs.partition(_ > 4))
  println(xs.mkString(", "))
  println(xs.mkString("<", "-", ">"))
  println(xs.sum)
  println(xs.max)
  println(xs.min)
  println(xs ++ List(100))
  println(0 :: xs)
  println(xs :+ 7)
  println(List(1, 2, 2, 3, 1).distinct)
  println(List(List(1), List(2, 3)).flatten)
  println(xs.flatMap(x => List(x, x)).take(4))
  println(List.fill(3)("z"))
  println(List.tabulate(4)(i => i * i))
  println(List.range(1, 5))
  println(xs(2))
  println(Nil.isEmpty)
  println(xs.headOption)
  println(List.empty[Int].headOption)

  val staff = List(
    Employee("Ann", "eng", 120),
    Employee("Bob", "eng", 100),
    Employee("Cid", "ops", 90)
  )
  println(staff.filter(_.dept == "eng").map(_.name))
  println(staff.map(_.salary).sum)
  println(staff.maxBy(_.salary).name)
  println(staff.sortBy(_.name).reverse.map(_.name))

  val pairs = for
    x <- List(1, 2, 3)
    y <- List("a", "b")
    if x != 2
  yield s"$x$y"
  println(pairs)

  val squares = for i <- 1 to 5 yield i * i
  println(squares)

  var total = 0
  for i <- 0 until 10 do total += i
  println(total)
  for
    i <- 1 to 2
    j <- 1 to 2
  do println(s"$i,$j")
  for (n, idx) <- List("x", "y").zipWithIndex do println(s"$idx=$n")

  val evens = for
    i <- 1 to 10
    sq = i * i
    if i % 2 == 0
  yield sq
  println(evens)

  println(Some(3).map(_ + 1))
  println(Option.empty[Int].map(_ + 1))
  println(Some(3).flatMap(x => if x > 2 then Some(x * 2) else None))
  println(Some(3).getOrElse(0))
  println(None.getOrElse(7))
  println(Some(2).filter(_ > 5))
  println(Some(1).orElse(Some(2)))
  println(Some(1).isDefined)
  println(Some("a").contains("a"))
  println(Some(4).fold(0)(_ * 2))
  println(Some(1).toList)
  println(parseAge("42"))
  println(parseAge("abc"))
  println(parseAge("200"))
  val combined = for
    a <- parseAge("20")
    b <- parseAge("22")
  yield a + b
  println(combined)

  println(safeDiv(10, 2))
  println(safeDiv(1, 0))
  println(safeDiv(10, 2).map(_ + 1))
  val chained = for
    a <- safeDiv(100, 5)
    b <- safeDiv(a, 2)
  yield a + b
  println(chained)
  println(safeDiv(1, 0).getOrElse(-1))
  println(safeDiv(8, 2).fold(e => e, v => v.toString))

  val t = (1, "two", 3.5)
  println(t)
  println(t._1)
  println(t._2)
  val (a, b) = (10, 20)
  println(a + b)
  val swapped = (1 -> "one")
  println(swapped)

  val str = "Hello, World"
  println(str.length)
  println(str.toUpperCase)
  println(str.substring(7))
  println(str.substring(0, 5))
  println(str.indexOf("World"))
  println(str.contains("lo, "))
  println(str.split(", ").toList)
  println(str.reverse)
  println(str.take(4))
  println(str.drop(7))
  println(str.startsWith("Hell"))
  println(str.replace("l", "L"))
  println(str(4))
  println(str.head)
  println(str.filter(_.isUpper))
  println(str.count(_ == 'l'))
  println("  padded ".trim)
  println("ab" * 3)
  println("a,b,,c".split(",").toList)
  println("42".toInt + 1)
  println("3.5".toDouble * 2)
  println("abc" < "abd")
  println("abc" == "abc")
  println(str.toList.take(3))
  println("""multi
    |line""".stripMargin)
  val sb = StringBuilder()
  sb.append("x").append(1).append('c')
  println(sb.toString)

  val arr = Array(3, 1, 2)
  arr(0) = 10
  println(arr(0) + arr.length)
  println(arr.toList)
  val grid = Array.fill(2)(Array.fill(3)(0))
  grid(1)(2) = 5
  println(grid(1).toList)
  println(Array.tabulate(4)(i => i * 2).toList)
