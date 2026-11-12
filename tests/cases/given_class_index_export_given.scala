// `given_import_index_export_forwarder` through a given wildcard: the forwarders
// `export Lib.given` makes in `Exp`, the forwarder of `Marker & A`
// (scalac 3.8.4: 13).
trait Marker
class A(val value: Int) extends Marker
object Lib:
  given mixed: (Marker & A) = new A(13)
object Exp:
  export Lib.given
object Fallback:
  given fallback: A = new A(99)
import Fallback.fallback
object Main:
  def main(args: Array[String]): Unit =
    import Exp.given
    println(summon[A].value)
