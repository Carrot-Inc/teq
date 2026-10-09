// An annotation's class and the names among its arguments use their imports.
object Lib {
  class Ann extends scala.annotation.StaticAnnotation
  class Arg(n: Int) extends scala.annotation.StaticAnnotation
  final val N = 3
  class Unused extends scala.annotation.StaticAnnotation
}
import Lib.Ann
import Lib.{Arg, N}
import Lib.Unused
@Ann class Use
class Params(@Arg(N) val x: Int)
