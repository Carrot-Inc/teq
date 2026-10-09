object F0:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 0).toString)
    List(make[Int]("f0", n => (n * 0).toString).show(0), x.show(1), y.show(2), stepper(0).next(3).toString)
