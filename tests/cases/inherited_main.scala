// An object's `main` inherited from a trait is the program's entry point, as scalac's static
// forwarder makes it; the object's own members and initialisation run first.
trait Launcher:
  def run(args: Array[String]): String
  def main(args: Array[String]): Unit =
    println("launching " + getClass.getSimpleName.stripSuffix("$"))
    println(run(args))

abstract class Named(val label: String) extends Launcher

object Main extends Named("app"):
  val greeting = "hello from " + label
  def run(args: Array[String]): String = greeting + " with " + args.length + " arguments"
