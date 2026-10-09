// jars: scala-library
// std: lean scala-library
// An object whose body throws fails its class's initialisation, the first time with the cause
// and from then on without the class; what it stored before the failure is what a reference
// made meanwhile saw.
object Seen:
  var last: String = "nothing"

object Failing:
  println("failing starts")
  Seen.last = "stored " + (Failing != null)
  val value: Int = if Seen.last.nonEmpty then throw new IllegalStateException("no value") else 1

@main def run(): Unit =
  try println(Failing.value)
  catch case e: ExceptionInInitializerError => println("failed: " + e.getCause.getMessage)
  try println(Failing.value)
  catch case e: NoClassDefFoundError => println("failed again")
  println(Seen.last)
