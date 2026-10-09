package quotes

object Main:
  def main(args: Array[String]): Unit =
    println(One.label + " " + One.show.show + " " + Two.label + " " + Two.show.show + " " + Two.again.show)
