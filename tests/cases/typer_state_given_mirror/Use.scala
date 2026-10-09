case class Point(x: Int, y: Int)
sealed trait Mode
case object Idle extends Mode

@main def run(): Unit =
  println(summon[Named[Point]].name)
  println(summon[Named[Mode]].name)
