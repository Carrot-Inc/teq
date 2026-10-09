object Handlers:
  def serve(f: ((Int, Int)) => Option[Int]): Option[Int] = f((1, 2))

  val handlers = List(
    serve { case (a, b) =>
      for
        x <- Some(a)
        y <- Some(b)
     yield x + y
    },
  )

  def classify(xs: List[Int]): List[String] = xs.map { case 0 =>
      "zero"
    case n if n > 0 =>
      "positive"
    case _ =>
      "negative"
  }

  def doubled(xs: List[Int]): List[Int] = xs.flatMap { x => for
      y <- List(x, x)
      z <- List(y * 2)
    yield z }

  val lastly = { () =>
      for x <- List(1)
    yield
    x + 1
  }

@main def run(): Unit =
  println(Handlers.lastly())
  println(Handlers.handlers)
  println(Handlers.classify(List(0, 3, -2)))
  println(Handlers.doubled(List(1, 2)))
