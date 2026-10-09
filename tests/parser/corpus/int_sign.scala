//> using platform js
@main def main(): Unit =
  println((-5).sign)
  println(0.sign)
  println(7.sign)
  println((-5).signum)
  println((-5L).sign)
  println(9L.sign)
  println(0L.signum)
  println((-2.5).sign)
  println(List(-3, 0, 4).map(_.sign))
