//> using platform jvm
// A method a class declares, found by its parameters' classes, a class
// whose name is no identifier among them (dotty's javaClassName,
// fullName.mangledString): the JDK's Class.getDeclaredMethod with the classes
// as its varargs array; a missing one the JDK's NoSuchMethodException.
class `Arg-X`
class Receiver:
  def accept(x: `Arg-X`, n: Int): String = "accepted"
@main def run(): Unit =
  val c = classOf[Receiver]
  println(c.getDeclaredMethod("accept", classOf[`Arg-X`], classOf[Int]).getName)
  try println(c.getDeclaredMethod("accept", classOf[`Arg-X`]))
  catch case e: NoSuchMethodException => println("no accept(Arg-X): " + e.getMessage)
