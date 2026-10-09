object Blocks:
  def evens(xs: List[Int]): Option[Int] = {
    val kept = xs.filter(_ % 2 == 0)
    kept.headOption
  }.map(_ * 10)

  def words: List[String] = {
    "a b c".split(" ").toList
  }.reverse

@main def run(): Unit =
  println(Blocks.evens(List(1, 2, 3)))
  println(Blocks.words)
  val typed = { 1 }.toString: String
  println(typed)
  println({ 2 }.toString match
    case "2" => "two"
    case _ => "other")
