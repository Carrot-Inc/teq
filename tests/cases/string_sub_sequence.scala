// String.subSequence, directly and through the CharSequence a String is.
@main def run(): Unit =
  val s = "teqé!"
  println(s.subSequence(1, 3))
  println(s.subSequence(3, 5).length)
  val cs: CharSequence = s
  println(cs.subSequence(0, 3).toString + cs.length)
  println(cs.subSequence(2, 2).length)
  try cs.subSequence(3, 1) catch case e: StringIndexOutOfBoundsException => println("out of bounds")
