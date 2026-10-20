object Z:
  val fourth = A.square(6)
  def fifth(k: Int): Shape = A.square(k + 1)

@main def run(): Unit =
  println(A.first.area)
  println(A.second.area)
  println(A.used)
  println(A.third.area)
  println(Z.fourth.area)
  println(Z.fifth(6).area)
