// expect: 15:17: error: No given instance of type TC[A] was found
// expect: 1 error found
// A plain inline call kept for the inlining phase keeps the site's scopes, not the typer's GADT
// bounds: the phase's context holds none (dotty's `Inlining` after the typer), so the case's
// `A =:= Int` gives `summonInline[TC[A]]` nothing, as scalac reports.
import scala.compiletime.summonInline
trait TC[A] { def v: String }
given TC[Int] with { def v = "int" }
inline def show[A]: String = summonInline[TC[A]].v
enum E[A]:
  case I extends E[Int]
  case S extends E[String]
object G:
  def f[A](e: E[A]): String = e match
    case E.I => show[A]
    case E.S => "s"
