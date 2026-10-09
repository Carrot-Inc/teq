// expect: 15:10: error: Unexpected pattern for summonFrom. Expected `x: T` or `_`
// expect: 18:10: error: Unexpected pattern for summonFrom. Expected `x: T` or `_`
// expect: 19:10: error: Unexpected pattern for summonFrom. Expected `x: T` or `_`
// expect: 3 errors found
// A case of `summonFrom` is `x: T`, `_: T` or `_`, scalac's rule: a bare binder, a binder over a
// typed pattern and an alternative are its E153 at the definition, called or not, each case
// reported (scalac 3.8.4 the same three, at the patterns). The call of `bare` adds nothing: the
// record failed, and its call is the plain call. A library body's `given x: T`, which TASTy
// pickles as `x @ (_: T)`, is no case of these: the definition check reads the program's
// sources only.
import scala.compiletime.summonFrom
given Int = 3
object Lib:
  inline def bare: String = summonFrom {
    case x => "any " + x
  }
  inline def bound: String = summonFrom {
    case x @ (_: Int) => "int " + x
    case _ | _ => "none"
  }
@main def run(): Unit = println(Lib.bare)
