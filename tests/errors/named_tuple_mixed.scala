// A tuple is named or plain, as under scalac.
object Main:
  def main(args: Array[String]): Unit =
    val mixed = (a = 1, 2)
    println(mixed)
// expect: Illegal combination of named and unnamed tuple elements
