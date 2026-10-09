object F4:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 4).toString)
    List(make[Int]("f4", n => (n * 4).toString).show(4), x.show(1), y.show(2), stepper(4).next(3).toString)
