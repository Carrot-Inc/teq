//> using file lib/Absent.scala
// A path that names nothing ends `teq interp` with exit 2 and the directive's place, before typing:
// the expectation is teq's message (Scala CLI's is "File not found").
object Main:
  def main(args: Array[String]): Unit = println("never")
