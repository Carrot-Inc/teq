object F2:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 2).toString)
    List(make[Int]("f2", n => (n * 2).toString).show(2), x.show(1), y.show(2), stepper(2).next(3).toString)
