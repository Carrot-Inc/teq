// teq: --werror
// A body typed on demand (the inherited result type of `conv` is read while `C`'s given index is
// built, before the walk reaches `C`) is typed under its class's `@nowarn` all the same.
import scala.language.implicitConversions
import scala.annotation.nowarn
trait Base:
  implicit def conv(x: Int): String
@nowarn
class C extends Base:
  implicit def conv(x: Int) =
    42
    x.toString
@main def main(): Unit = println(new C().conv(7))
