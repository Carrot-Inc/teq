object F3:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 3).toString)
    List(make[Int]("f3", n => (n * 3).toString).show(3), x.show(1), y.show(2), stepper(3).next(3).toString)
