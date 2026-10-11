// A conversion's implicit scope is its argument's widened type's and the expected type's as it
// is (dotty's `wildProto`, a `ViewProto` of `argument.tpe.widen` and `pt`): `O.x.type`, `x`
// inherited from a trait, brings `O`'s conversion to it, as `O.U.type` of a nested object does.
import scala.language.implicitConversions
class Foo
trait A:
  val x: Foo = new Foo
  object U
object O extends A:
  given cv: Conversion[Int, x.type] with
    def apply(i: Int): x.type = x
  given cu: Conversion[String, U.type] with
    def apply(s: String): U.type = U
@main def run(): Unit =
  val y: O.x.type = 1
  println(y eq O.x)
  val z: O.U.type = "u"
  println(z eq O.U)
