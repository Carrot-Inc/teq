//> using platform jvm
// A value class's own parameter selected on a null of the class's type is a NullPointerException where the value
// is unboxed. The JVM's platform: JavaScript reads a member of null with a TypeError.
class V(val i: Int) extends AnyVal

object Main:
  def missing(): Int =
    null.asInstanceOf[V]
      .i

  def main(args: Array[String]): Unit =
    try println(missing())
    catch case _: NullPointerException => println("NullPointerException")
