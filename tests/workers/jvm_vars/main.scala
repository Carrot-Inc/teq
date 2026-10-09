@main def run(): Unit =
  println("n: " + List(Fx.decoded.fold(_ => -1, _.length), Fx.decoded.fold(_ => -1, _.length)).mkString(","))
