@main def run(): Unit =
  val matrix = List(List(1, 2), List(3, 4))
  println(matrix.transpose.map(_.max))
  println(matrix.transpose.map(_.min))
