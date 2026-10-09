// Under `import language.experimental.relaxedLambdaSyntax` a lambda after `:` may stand on the
// colon's line (dotty's `Parsers.followingIsLambdaAfterColon`, case 1b), its body continuing on an
// indented line; without the import only the indented body is a colon lambda
// (tests/errors/colon_lambda_single_line.scala).
import language.experimental.relaxedLambdaSyntax

def fun(f: Int => Int => Int): Int = f(1)(2)
def twice(f: Int => Int): Int = f(f(1))

@main def run() =
  val a = fun: (x: Int) =>
    (y: Int) => x + y
  val b = fun: (x: Int) => (y: Int) => x + y
  val c = fun: (x: Int) => (y: Int) =>
    x * 10 + y
  val d = twice: x => x + 3
  println(List(a, b, c, d))
