// `Class.forName` as the JDK's contract has it: an object's class initialised only when asked, which
// constructs the instance; a primitive's name refused; an array class by its descriptor; a class
// the program defines under a computed name; the mirror class of a top-level object without a
// companion class, which initialises nothing.
object Hidden:
  println("initialized")

class Plain

class Paired
object Paired:
  println("paired initialized")

object Main:
  def main(args: Array[String]): Unit =
    val cl = classOf[Main.type].getClassLoader
    println(Class.forName("Hidden$", false, cl).getName)
    println(Class.forName("Hidden$").getName)
    try println(Class.forName("int")) catch case _: ClassNotFoundException => println("missing int")
    println(Class.forName("[I").getName)
    val name = "Pla" + "in"
    println(Class.forName(name) == classOf[Plain])
    try Class.forName("Missing") catch case e: ClassNotFoundException => println(e.getMessage)
    println(Class.forName("Hidden").getName)
    println(Class.forName("Paired").getName)
    println(Class.forName("Paired$", false, cl).getName)
    println(Class.forName("Paired$").getName)
