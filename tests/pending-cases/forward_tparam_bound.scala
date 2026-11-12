// A class's type parameter bounded by one its clause
// declares after it (`[L <: U, U]`, chimney-macro-commons'
// `Existential.Bounded`): teq is to write the class's
// TASTy, in a build writing products; a part of its own.
sealed trait Bounded[L <: U, U, F[_ >: L <: U]]:
  def value: F[L]
final case class Impl[L <: U, U, F[_ >: L <: U]](value: F[L]) extends Bounded[L, U, F]

@main def run(): Unit = println(Impl[Int, Any, List](List(1)).value)
