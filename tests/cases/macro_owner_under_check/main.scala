// A macro run from inside the check of the object that calls it: its inline method is the
// object's own, or another object's reached through one of the object's own. The run makes the
// module of the class under check, whose body has not run yet.
object Test:
  inline def twice(inline x: Int): Int = ${ Macros.twice('x) }
  inline def greeting = Dsl.shout("teq")

  val label = greeting

  def main(args: Array[String]): Unit =
    println(twice(21))
    println(label)
    println(greeting.length)
