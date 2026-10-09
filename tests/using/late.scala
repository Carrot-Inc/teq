// A directive after the first code is ignored, as Scala CLI ignores it with a warning: the
// program prints its own value, and Helper, never included, would be a missing name.
object Main:
  def main(args: Array[String]): Unit = println("no inclusion after the code")
//> using file lib/Absent.scala
