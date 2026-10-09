@main def run(): Unit =
  val c = 'a'
  println(c + 1)
  println(c + 'b')
  println((c + 1).toChar)
  println(c.toInt)
  println(c < 'b')
  println("abc"(1))
  println("abc".charAt(1) + 1)
  val s = "x" + c
  println(s)
  println(c.isLetter)
  println(('a' to 'e').mkString)
  val x: Any = c
  println(x == "a")
  println(x.isInstanceOf[Char])
  println(x.isInstanceOf[String])
