// jars: scala-library cats-core cats-kernel
// A covariant abstract constructor read from a jar keeps its declared variance: the non-empty chain of
// nothing is a chain of anything, a tuple of validations joins its error types, and the alias-typed
// instance is found for the constructor.
//> using scala 3.8.4
//> using dep org.typelevel::cats-core:2.13.0
package chains
import cats.data.{NonEmptyChain, ValidatedNec}
import cats.syntax.all.*

enum Problem:
  case Missing

object Use:
  def widen[E](x: NonEmptyChain[Nothing]): NonEmptyChain[E] = x
  def both(name: String, age: Option[Int]): ValidatedNec[Problem, (String, Int)] =
    (name.validNec, age.toValidNec(Problem.Missing)).mapN((n, a) => (n, a))
  def listed(errors: NonEmptyChain[String]): List[String] = errors.toList
