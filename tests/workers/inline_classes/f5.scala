object F5:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 5).toString)
    List(make[Int]("f5", n => (n * 5).toString).show(5), x.show(1), y.show(2), stepper(5).next(3).toString)
