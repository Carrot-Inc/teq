package plot.use

@main def run(): Unit =
  println(First01.run(1) + First02.run(2) + First03.run(3))
  println(Early01.picked + Early10.sum(List(1, 2, 3)))
