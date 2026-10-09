// expect: type argument C does not have the same kind as its bound
// expect: type argument Int does not conform to upper bound [_] =>> Any
// expect: type argument Map does not have the same kind as its bound
// expect: type List takes type arguments
object T:
  class C[F[_]]
  val x: C[C] = ???
  def f[F[_]] = 1
  def g = f[Int]
  def k = f[Map]
  val l: List[List] = ???

@main def run(): Unit = println(1)
