// A JDK class the standard library stands in for keeps the static nested classes the standard
// library leaves out, read from the JDK's class files as a class of its own would be.
object Main:
  def main(args: Array[String]): Unit =
    println(java.util.Locale.Category.FORMAT.name())
    println(java.lang.Character.UnicodeBlock.of('a'))
    val e: java.util.Map.Entry[String, Int] = new java.util.AbstractMap.SimpleEntry[String, Int]("k", 3)
    println(e)
