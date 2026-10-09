// `sliding` and `grouped` of an array give arrays, which a pattern of an array matches (zio-test
// parses its arguments with `args.sliding(2, 2).collect { case Array("-t", term) => .. }`).
@main def run(): Unit =
  val args = Array("-t", "x", "-tags", "y", "z")
  println(args.sliding(2, 2).map(_.mkString("[", ",", "]")).toList)
  println(args.sliding(2, 2).collect {
    case Array("-t", term) => "term " + term
    case Array("-tags", tag) => "tag " + tag
  }.toList)
  println(args.grouped(2).map(_.length).toList)
  println(Array(1, 2, 3, 4).sliding(2).map(_.sum).toList)
  println(Array(1, 2, 3, 4).sliding(3, 1).map(_.toList).toList)
  val windows: Iterator[Array[Int]] = Array(1, 2, 3).grouped(2)
  println(windows.map(_.reverse.mkString).toList)
