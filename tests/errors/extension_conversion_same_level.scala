// expect: 12:45: error: Found:    Int
// expect: Required: String
// expect: 1 error found
import scala.language.implicitConversions

class Wrapper { def f: String = "ok" }
class Num
object Num {
  extension (x: Num) def f = 1
  given Conversion[Num, Wrapper] = _ => new Wrapper
}
object Use { val result: String = (new Num).f }
