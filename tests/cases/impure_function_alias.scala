// `scala.ImpureFunctionN` and its kin are `Definitions.newFunctionNType`'s aliases (Definitions.scala 113-129), not
// classes: the function type of the name without `Impure` with its retaining annotation, which capture checking alone
// reads. A function converts to the alias and back, whatever its arity's
// spelling.
package impure_function_alias

object Impure:
  def f(x: scala.ImpureFunction1[Int, Int]): Int => Int = x
  def g(x: Int => Int): scala.ImpureFunction1[Int, Int] = x
  def h(x: scala.ImpureFunction0[String]): () => String = x
  def k(x: scala.ImpureFunction2[Int, Int, Int]): scala.Function2[Int, Int, Int] = x
  type I = Int
  def l(x: scala.Function023[I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I]): scala.ImpureFunction023[I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I, I] = x

@main def main(): Unit =
  val inc: Int => Int = _ + 1
  println(Impure.f(Impure.g(inc))(41))
  println(Impure.h(() => "zero")())
  println(Impure.k(_ * _)(6, 7))
  val sum: scala.Function023[Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I, Impure.I] = _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _ + _
  println(Impure.l(sum)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
