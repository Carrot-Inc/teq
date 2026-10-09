// expect: 8:11: error: type argument Int does not have the same kind as its parameter F[_]
// expect: 9:11: error: type argument Map does not have the same kind as its parameter F[_]
// expect: 10:15: error: type argument List does not have the same kind as its parameter A
// expect: 3 errors found
object T:
  class C[F[_]]
  def f[F[_]] = 1
  def g = f[Int]
  def k = f[Map]
  val l: List[List] = ???
  def ok = f[List]
  val fine: C[Option] = ???
  def nothing: C[Nothing] = ???

@main def run(): Unit = println(1)
