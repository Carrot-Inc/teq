//> using platform jvm
// A method found by its parameter's class, a class whose name is no identifier: the class literal and the method's
// erased parameter are the one name the JVM gives the class (dotty's javaClassName, fullName.mangledString), so
// getMethod finds it as the JDK's does.
class `Arg-X`
class Receiver:
  def accept(x: `Arg-X`): String = "accepted"
@main def run(): Unit =
  val c = classOf[Receiver]
  println(classOf[`Arg-X`].getName)
  try println(c.getMethod("accept", classOf[`Arg-X`]).invoke(new Receiver(), new `Arg-X`()))
  catch case e: NoSuchMethodException => println("missing")
