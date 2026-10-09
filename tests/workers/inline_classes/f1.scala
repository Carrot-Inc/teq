object F1:
  def shows: List[String] =
    val (x, y) = twice[Int](n => (n + 1).toString)
    List(make[Int]("f1", n => (n * 1).toString).show(1), x.show(1), y.show(2), stepper(1).next(3).toString)
