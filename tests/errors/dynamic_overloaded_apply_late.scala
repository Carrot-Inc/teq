import scala.language.dynamics
class Typed extends Dynamic:
  def applyDynamic(name: String)(args: Int*): Int = 1
  def applyDynamic(name: String)(arg: String): Int = 2
def uses(): Unit =
  val x: Int = "s"
  println(x)
// The restriction on an overloaded `applyDynamic` is `RefChecks`' (`checkOverloadedRestrictions`): where the typer
// has reported an error it is not told, as under scalac.
// expect: 6:16: error: Found:    "s"
// expect+ Required: Int
// expect: 1 error found
// absent: implementation restriction
