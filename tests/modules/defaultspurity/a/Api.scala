package dpur

// An object whose body initialises, and one whose body does not: `TreeInfo.refPurity` reads the
// first as an idempotent path and the second as a path once its class is complete, from the
// products too.
object Inits:
  val v = { print("I"); 1 }
  override def toString = "O"

object Plain:
  override def toString = "P"

object Api:
  def f(a: Int, b: Any = 0)(c: Int = 2): String = s"$a$b$c"
