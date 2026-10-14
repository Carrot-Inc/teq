//> using platform jvm
// An array's index outside it throws `ArrayIndexOutOfBoundsException`, reading or writing, as the JVM's
// `iaload`/`iastore` do. JavaScript's arrays are unchecked, as Scala.js's in production, so this runs in the
// interpreter and on the JVM alone.
object Main:
  def main(args: Array[String]): Unit =
    val a = Array(1, 2)
    try { a(2) = 3; println(a.length) } catch case e: ArrayIndexOutOfBoundsException => println("write " + e.getMessage)
    val c = Array('a')
    try { c(-1) = 'b'; println(c.length) } catch case e: ArrayIndexOutOfBoundsException => println("write " + e.getMessage)
    try println(a(5)) catch case e: ArrayIndexOutOfBoundsException => println("read " + e.getMessage)
    try println(c(-2)) catch case e: IndexOutOfBoundsException => println(e.getClass.getName)
