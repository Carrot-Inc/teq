// A transparent inline method declared `Any` whose match joins a binder's `Int` with a constant
// branch: the expansion's type is the cases' lub (`typedMatch`), `Int`, not the declared `Any`,
// so the overload taking an `Int` is chosen at the call; a branch of another class keeps the
// union of the cases (`if`'s lub alike).
import scala.annotation.targetName

object Up:
  transparent inline def first(t: Any): Any = t match
    case a: Int => a
    case _ => 0
  transparent inline def either(c: Boolean): Any = if c then 1 else 2
  transparent inline def mixed(t: Any): Any = t match
    case a: Int => a
    case _ => "none"

def pick(x: Any): String = "Any"
def pick(x: Int): String = "Int"
def pickU(x: Any): String = "Any"
@targetName("pickU2") def pickU(x: Int | String): String = "Int | String"

@main def main(): Unit =
  println(pick(Up.first(5)))
  println(pick(Up.first("s")))
  println(pick(Up.either(true)))
  println(pickU(Up.mixed(5)))
  println(pickU(Up.mixed('c')))
