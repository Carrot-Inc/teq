// Without `relaxedLambdaSyntax` a lambda after `:` needs its body on an indented line; on the
// colon's line the `:` is an ascription, as scalac 3.8.4 reads it
// (tests/cases/colon_lambda_relaxed.scala has the feature on).
def fun(f: Int => Int => Int): Int = f(1)(2)
def run() =
  val b = fun: (x: Int) => (y: Int) => x + y
// expect: 6:40: error:
