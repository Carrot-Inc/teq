// expect: 7:23: error: e is already defined as parameter e
// expect: 9:15: error: n is already defined as parameter n
// expect: 11:21: error: n is already defined as parameter n
// expect: 3 errors found
object Syntax:
  extension (e: Either.type)
    def leftNec[E, A](e: E): Either[List[E], A] = Left(List(e))
  extension (n: Int)
    def times(n: Int): Int = n * n
    def plus(m: Int)(k: Int): Int = m + k + n
  def twice(n: Int)(n: Int): Int = n

@main def run(): Unit = println(Syntax.twice(1)(2))
