// An extension attempt infers Num.f, then gives way to a conversion: the inferred body's use of
// its import stays.
import scala.language.implicitConversions
import Conversions.given
object Use { val result: String = (new Num).f }
class Wrapper { def f: String = "ok" }
object Conversions {
  given Conversion[Num, Wrapper] = _ => new Wrapper
}
