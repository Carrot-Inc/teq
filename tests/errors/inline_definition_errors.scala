// An inline method's body typed at its definition, called or not, as scalac 3.8.4 types it:
// a member or an extension that the parameter's declared type lacks, an error
// in an uncalled body, in either branch of an `inline if`, a case of an `inline match` against
// the abstract result type, a search for a given at the definition, a `return`, and a local
// inline method's body, typed where the block defines it.
// expect: value only is not a member of B
// expect: value extra is not a member of B
// expect: value nosuch is not a member of Int
// expect: value nosuch is not a member of String
// expect: type mismatch: found Int, required T
// expect: no given instance of type TC[(x : B)] was found
// expect: No explicit return allowed from inlineable method returns
// expect: value nosuchLocal is not a member of Int
import scala.compiletime.erasedValue
trait B
class C extends B:
  def only: Int = 1
extension (c: C) def extra: Int = 2
trait TC[A]
given TC[B] with {}
inline def member(x: B): Int = x.only
inline def extended(x: B): Int = x.extra
inline def uncalled(x: Int): Int = x.nosuch
inline def branches(inline b: Boolean): Int = inline if b then 1 else "no".nosuch
inline def erased[T]: T = inline erasedValue[T] match { case _: Int => 0 }
inline def searched(x: B): TC[x.type] = summon[TC[x.type]]
inline def returns(x: Int): Int = { if x > 0 then return 1; 2 }
def host(): Int =
  inline def local(y: Int): Int = y.nosuchLocal
  1
@main def run(): Unit = println(member(new C))
