
@main def hello(): Unit =
  println("hello " + List(1, 2, 3).map(_ * 2).sum + " " + 1.5 + " " + java.lang.Long.toHexString(255L))
