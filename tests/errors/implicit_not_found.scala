// A missing given is reported with the `@implicitNotFound` text of its class, the class's type
// parameters `${A}` replaced by the arguments (`?X` for a name that is none), or of a base class,
// its parameters seen from the type (dotty's `MissingImplicitArgument.userDefinedImplicitNotFoundTypeMessage`),
// looked up through an intersection's sides, an alias and a bound; the using parameter's own
// annotation comes first (`userDefinedImplicitNotFoundParamMessage`), formatted with the type
// parameters of the classes around the method as the receiver gives them (`formatAnnotationMessage`,
// `substitutableTypeSymbolsInScope`), and a text that starts with `explain=` is no message.
import scala.annotation.implicitNotFound
@implicitNotFound("Need evidence for ${A}")
trait Evidence[A]
@implicitNotFound("No codec between ${In} and ${Out} (${Missing})")
trait Codec[In, Out]
trait Sub[X] extends Evidence[X]
trait Plain[A]
trait Extra
@implicitNotFound("explain=Vaults come from the bank")
trait Vault
type Alias = Evidence[Option[String]]
def meet(using t: Evidence[List[Int]] & Extra): Int = 1
def second(using t: Extra & Evidence[Int]): Int = 1
def aliased(using t: Alias): Int = 1
def own[T](using @implicitNotFound("Supply ${T} once") t: Evidence[T]): Int = 1
def vault(using v: Vault): Int = 1
def bounded[T <: Evidence[Long]](using t: T): Int = 1
class Box[T](val t: T)(using @implicitNotFound("Box needs evidence of ${T}") e: Evidence[T])
class Service[A]:
  def use(using @implicitNotFound("Owner token ${A}") t: Plain[A]): Int = 1
def run() =
  summon[Evidence[Int]]
  summon[Codec[String, List[Int]]]
  summon[Sub[Long]]
  summon[Plain[Int]]
  meet
  second
  aliased
  own[Char]
  vault
  bounded
  Box(1)
  new Service[Int].use
// expect: 29:24: error: Need evidence for Int
// expect: 30:35: error: No codec between String and List[Int] (?Missing)
// expect: 31:20: error: Need evidence for Long
// expect: 32:21: error: no given instance of type Plain[Int] was found for parameter x
// expect: 33:7: error: Need evidence for List[Int]
// expect: 34:9: error: Need evidence for Int
// expect: 35:10: error: Need evidence for Option[String]
// expect: 36:12: error: Supply Char once
// expect: 37:8: error: no given instance of type Vault was found for parameter v
// expect: 38:10: error: Need evidence for Long
// expect: 39:9: error: Box needs evidence of Int
// expect: 40:23: error: Owner token Int
