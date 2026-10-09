// String.lastIndexOf with a starting index searches backwards from it, and a negative index finds
// nothing; Character.forDigit is a digit's lower-case character, or '\u0000' outside the radix.
@main def run(): Unit =
  val s = "hello, world"
  println(s.lastIndexOf('o', 12) + " " + s.lastIndexOf('o', 7) + " " + s.lastIndexOf('o', 4) + " " + s.lastIndexOf('o', 3))
  println(s.lastIndexOf('h', 0) + " " + s.lastIndexOf('h', -1) + " " + s.lastIndexOf('z', 100))
  println(s.lastIndexOf("l", 9) + " " + s.lastIndexOf("lo", 3) + " " + s.lastIndexOf("", 5) + " " + s.lastIndexOf("", -3) + " " + s.lastIndexOf("", 99))
  println("aaa".lastIndexOf("aa", 0) + " " + "aaa".lastIndexOf("aa", 5) + " " + "é,é".lastIndexOf('é', 1))
  println((0 until 16).map(d => Character.forDigit(d, 16)).mkString)
  println(Character.forDigit(35, 36) + " " + (Character.forDigit(10, 10).toInt) + " " + Character.forDigit(1, 37).toInt + " " + Character.forDigit(-1, 10).toInt + " " + Character.forDigit(1, 1).toInt)
