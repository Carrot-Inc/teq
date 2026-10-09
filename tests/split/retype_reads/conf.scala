package reads

/** What the macro reads without calling a def of this file: an object's val, a top-level val
  * and the initialiser of a class it constructs. */
object Config:
  val prefix: String = "app-"

val suffix: String = "-x"

final class Wrap(val s: String):
  val shown: String = "<" + s + ">"
