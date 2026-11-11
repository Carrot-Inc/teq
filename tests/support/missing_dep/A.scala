// Compiled by scalac against dep/B.scala, into a jar without it (tests/support/jars.sh `missingdep_lib_jar`): inline
// bodies that name a class their user's class path does not hold.
package mdlib
object A:
  inline def inl(x: Int): Int = x + new dep.Dep().v
  def plain(x: Int): Int = x + 1
class Ops(val s: String):
  inline def h2(b: Int = 4, inline a: Int = new dep.Dep().v): Int = b * 10 + a + s.length
object Fw:
  extension (s: String)
    def ops: Ops = Ops(s)
    export ops.h2
object B:
  inline def d8(a: Int, b: Int = 3): Int = a + b + new dep.Dep().v
trait Base:
  def tl[N <: Tuple, V <: Tuple](x: NamedTuple.NamedTuple[N, V]): Any
object Impl extends Base:
  inline def tl[N <: Tuple, V <: Tuple](x: NamedTuple.NamedTuple[N, V]): NamedTuple.Tail[NamedTuple.NamedTuple[N, V]] = x.toTuple.drop(1)
