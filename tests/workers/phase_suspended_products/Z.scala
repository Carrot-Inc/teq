object Z:
  val fourth = A.square(5)
  val fifth = LocalMacro.run
  def sixth(k: Int): Shape = A.square(A.twice(k))

@main def run(): Unit =
  println(A.used)
  println(A.first.area)
  println(A.second.area)
  println(A.third(1).area)
  println(Z.fourth.area)
  println(Z.fifth)
  println(Z.sixth(3).area)
