//> using platform jvm
// `Class.getDeclaredMethod`, which a library calls to find a method by reflection: one the class
// declares, with parameters or none, invoked; one it inherits, or one of other parameters, a
// `NoSuchMethodException` naming the class, the method and the parameters' classes.
class Greeter:
  def greet(name: String): String = "hello " + name
  def bare(): String = "bare"

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    val greet = classOf[Greeter].getDeclaredMethod("greet", classOf[String])
    show("declared")(greet.getName + " " + greet.invoke(new Greeter, "you"))
    show("declared, no parameters")(classOf[Greeter].getDeclaredMethod("bare").invoke(new Greeter))
    show("inherited")(classOf[Greeter].getDeclaredMethod("toString"))
    show("other parameters")(classOf[Greeter].getDeclaredMethod("greet"))
    show("more parameters")(classOf[Greeter].getDeclaredMethod("bare", classOf[String], classOf[Int]))
