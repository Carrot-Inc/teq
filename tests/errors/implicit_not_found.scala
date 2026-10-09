// A missing given is reported with the `@implicitNotFound` text of its class, the class's type
// parameters `${A}` replaced by the arguments (`?X` for a name that is none), or of a base class,
// its parameters seen from the type (dotty's `MissingImplicitArgument.userDefinedImplicitNotFoundTypeMessage`).
import scala.annotation.implicitNotFound
@implicitNotFound("Need evidence for ${A}")
trait Evidence[A]
@implicitNotFound("No codec between ${In} and ${Out} (${Missing})")
trait Codec[In, Out]
trait Sub[X] extends Evidence[X]
trait Plain[A]
def run() =
  summon[Evidence[Int]]
  summon[Codec[String, List[Int]]]
  summon[Sub[Long]]
  summon[Plain[Int]]
// expect: 12:24: error: Need evidence for Int
// expect: 13:35: error: No codec between String and List[Int] (?Missing)
// expect: 14:20: error: Need evidence for Long
// expect: 15:21: error: no given instance of type Plain[Int] was found for parameter x
