@main def run(): Unit =
  println(addOne(41))
  val pair = (addOne(1), addOne(2))
  println(pair)
  def twice = addOne(addOne(0))
  println(twice)
  println(thunk())
  val base = 40
  println(joiner(base)(2, "n="))
