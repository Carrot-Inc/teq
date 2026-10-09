import scala.collection.mutable.ArrayBuffer

sealed trait Opt
case class IconBefore(icon: String) extends Opt
case class IconAfter(icon: String) extends Opt
case object Small extends Opt

var evaluated = 0
def even(n: Int): Boolean =
  evaluated += 1
  n % 2 == 0

@main def main(): Unit =
  val xs = List(Some(1), None, Some(3))
  println(xs.collect { case Some(x) => x })
  println(Vector(1, 2, 3, 4).collect { case n if n % 2 == 0 => n * 10 })
  println(Seq("a", "bb", "ccc").collect { case s if s.length > 1 => s.toUpperCase })
  println(Set(1, 2, 3).collect { case 2 => "two" })
  println(Map("a" -> Some(1), "b" -> None).collect { case (k, Some(v)) => k -> v })
  println(Map(1 -> "a").collect { case (k, v) => (v, k) })
  println(Option(5).collect { case n if n > 3 => n + 1 })
  println(Option(1).collect { case n if n > 3 => n + 1 })
  println((None: Option[Int]).collect { case n => n })
  println(Array(1, 2, 3).collect { case 2 => "two" }.toList)
  println(Array(1, 2, 3).collectFirst { case n if n > 1 => n })
  println(Iterator.from(1).collect { case n if n % 3 == 0 => n }.take(3).toList)
  println(List(1, 2, 3).iterator.collectFirst { case 2 => "two" })
  println(ArrayBuffer(1, 2, 3).collect { case 3 => 30 })
  println((1 to 10).collect { case n if n > 8 => n })
  println(List(1, 2, 3).collectFirst { case n if n > 1 => n * 2 })
  println(List(1, 2, 3).collectFirst { case n if n > 5 => n })
  println(Vector("a", "b").collectFirst { case "b" => 2 })
  println(Set(1, 2).collectFirst { case n if n > 1 => n })
  println(Map(1 -> "a", 2 -> "b").collectFirst { case (k, "b") => k })

  val opts: List[Opt] = List(Small, IconBefore("l"), IconAfter("t"))
  println(opts.collectFirst { case IconBefore(i) => i })
  println(opts.collectFirst { case s: IconAfter => s })
  println(opts.collectFirst { case Small => "small" })
  println(List(1, 2).map(x => (x, x)).collect { case (a, b) if a == b => a + b })

  println(List(1, 2, 3, 4).collect { case n if even(n) => n })
  println(evaluated)
  println(List(1, 2, 3, 4).collectFirst { case n if even(n) => n })
  println(evaluated)

  val nested = List(List(1, 2), List(3))
  println(nested.collect { case x :: rest if rest.nonEmpty => x })
  println(List("100", "regular", "400", "700italic").collect {
    case "100" => 100
    case "regular"
       | "400" => 400
    case "700" => 700
  }.filter(w => w >= 400).sorted)
  val grouped = List("a", "bb", "cc", "ddd").groupBy(_.length)
  val big = grouped
    .collect:
      case (len, vs) if len > 1 => (len, vs.size)
    .toList
    .sortBy(_._1)
  println(big)
  println(List("x", "7y").collectFirst:
    case v if v.startsWith("7") => v.length
  )
