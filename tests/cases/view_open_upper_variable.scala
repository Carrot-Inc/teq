// A conversion's variable left open for the member (`EitherOps[A, B]` of an `implicit def` over
// `Right[Nothing, String]`, `A` held nonvariantly) and bounded above by the member's own
// (`ensure[AA >: A]`): `interpolateTypeVars` instantiates the `A` that the result does not hold
// from above (`fromBelow = hasLowerBound`, nothing below it), to `fullUpperBound`, which keeps the
// parameter `AA` (`maxUpper`), not `Any`; `AA` then takes the failure's type from below, as
// scalac's `ensure[Error.CodeVerifierTooShort.type]`.
package viewopenuppervariable

import scala.language.implicitConversions

sealed trait Error
object Error:
  case object CodeVerifierTooShort extends Error
  case object Other extends Error

trait OpaqueString[A]:
  def apply(s: String): A
opaque type CodeVerifier = String
object CodeVerifier extends OpaqueString[CodeVerifier]:
  def apply(s: String): CodeVerifier = s

final class EitherOps[A, B](private val eab: Either[A, B]) extends AnyVal:
  def ensure[AA >: A](onFailure: => AA)(f: B => Boolean): Either[AA, B] = eab match
    case Left(_) => eab
    case Right(b) => if f(b) then eab else Left(onFailure)
implicit def catsSyntaxEither[A, B](eab: Either[A, B]): EitherOps[A, B] = new EitherOps(eab)

def from(value: String): Either[Error, CodeVerifier] =
  Right(value)
    .ensure(Error.CodeVerifierTooShort)(_.size >= 43)
    .map(CodeVerifier(_))

def kept(value: String): Either[Error, Int] =
  val checked = Right(value).ensure(Error.Other)(_.nonEmpty)
  checked.map(_.length)

@main def main(): Unit =
  println(from("short"))
  println(from("x" * 43).map(_.length))
  println(kept(""))
  println(kept("four"))
