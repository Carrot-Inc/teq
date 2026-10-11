package equalsalias

object Main:
  def main(args: Array[String]): Unit =
    println(Aliased(new Touch, 1L) == Aliased(new Touch, 2L))
    println(Primitive(new Touch, 1L) == Primitive(new Touch, 2L))
