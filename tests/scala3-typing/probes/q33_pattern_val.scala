@main def run(): Unit =
  val opt: Option[Int] = Some(1)
  val Some(x) = opt
  println(x)
  val (a, b) = (1, 2)
  println(a)
  val h :: t = List(1, 2)
  println(h)
