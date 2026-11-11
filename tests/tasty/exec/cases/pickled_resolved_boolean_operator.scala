// A method value of a Boolean operator, whose eta-expansion applies the operator's symbol to a parameter of
// another type than its declared one (`B <: Boolean`): the selection is pickled with the symbol's signature,
// `&&(Boolean): Boolean`, and the inline body is written.
object Lib:
  inline def and[B <: Boolean](b: Boolean): B => Boolean = b.&&
  inline def or(b: Boolean): Boolean => Boolean = b.||

@main def run(): Unit =
  val conj = Lib.and[Boolean](true)
  val disj = Lib.or(false)
  println(conj(true))
  println(conj(false))
  println(disj(true))
  println(disj(false))
