// A wildcard argument is bounded by its type parameter's declared bounds too (`TypeComparer.isSubArg`: `tp1 & paramBounds(tparam)`):
// a `Cfg[?]` of `Cfg[F <: Flags]` is a `Cfg[? <: Flags]`, as a value and as a given.
sealed trait Flags
final class On extends Flags
final class Off extends Flags
class Cfg[F <: Flags](val name: String)
object A:
  given Cfg[?] = Cfg[On]("on")
def pick(using c: Cfg[? <: Flags]): String = c.name
@main def run(): Unit =
  import A.given
  println(pick)
  val c: Cfg[?] = Cfg[Off]("off")
  val d: Cfg[? <: Flags] = c
  println(d.name)
