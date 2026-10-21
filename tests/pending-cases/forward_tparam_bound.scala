// A class's type parameter bounded by one its clause declares after it (`[L <: U, U]`, chimney-macro-commons'
// `Existential.Bounded`): teq writes no TASTy of the class ("cannot write TASTy for a reference to the type parameter U
// outside its scope"), so a build writing products refuses the program: master's, a part of its own (found by the fold
// of the gate of `bb04d0aaf`; `[U, L <: U]` pickles).
sealed trait Bounded[L <: U, U, F[_ >: L <: U]]:
  def value: F[L]
final case class Impl[L <: U, U, F[_ >: L <: U]](value: F[L]) extends Bounded[L, U, F]

@main def run(): Unit = println(Impl[Int, Any, List](List(1)).value)
