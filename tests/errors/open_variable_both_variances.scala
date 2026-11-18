// expect: 14:16: error: type mismatch: found Any => Nothing, required Int
// expect: 15:22: error: type mismatch: found (Any => Int, Nothing), required List[Int]
// expect: 16:19: error: type mismatch: found Fn[Any, Nothing], required String
// A value that does not conform, whose type holds a variable in a contravariant and in a covariant place with
// nothing below it: the variable is not defined further (the reference's `canDefineFurther` decides it again
// at the covariant place and fails), so the message names it by its variance at each place, as scalac's does
// ("Found: Any => Nothing"); defined at its first place alone it would read `Any => Any`.
object OpenVariable {
  trait Fn[-A, +B]
  def idf[T]: T => T = x => x
  def pairf[T]: (T => Int, T) = ???
  def both[T]: Fn[T, T] = ???

  val a: Int = idf
  val b: List[Int] = pairf
  val c: String = both
}
