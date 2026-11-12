// `wrongNumberOfTypeArgs`' message (E023): the constructor as its
// `TypeRef` is shown (`V.L`, `h.L`, `H.this.L` for an alias whose
// right side is a lambda), and `actual:` lists the argument trees as
// written, as dotty's printer shows an untyped tree (`(Int) => Int`,
// ` <: Int`, `[X] =>> X`). scalac's columns are 0-based.
trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type S = String
  type A1 = Aux[S, Int => Int]
  type A3 = Aux[? <: Int, List[S]]
  type A4 = Aux[Predef.String, [X] =>> X]
  type L = [X, Y] =>> Map[Y, X]
  type B1 = L[Int]
trait H:
  type L[X, Y] = Map[Y, X]
  def x: L[Int]
def f(h: H)(y: h.L[Int]): Int = 1
// expect: wrong_number_of_type_args_message_forms.scala:10:13: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: actual:   [S, (Int) => Int]
// expect: wrong_number_of_type_args_message_forms.scala:11:13: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: actual:   [ <: Int, List[S]]
// expect: wrong_number_of_type_args_message_forms.scala:12:13: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: actual:   [Predef.String, [X] =>> X]
// expect: wrong_number_of_type_args_message_forms.scala:14:13: error: Not enough type arguments for V.L[X, Y]
// expect: actual:   [Int]
// expect: wrong_number_of_type_args_message_forms.scala:17:10: error: Not enough type arguments for H.this.L[X, Y]
// expect: wrong_number_of_type_args_message_forms.scala:18:16: error: Not enough type arguments for h.L[X, Y]
// expect: 6 errors found
