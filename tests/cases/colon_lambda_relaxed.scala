// Under `import language.experimental.relaxedLambdaSyntax` a lambda after `:` may stand on the
// colon's line (dotty's `Parsers.followingIsLambdaAfterColon`, case 1b), its body continuing on an
// indented line; without the import only the indented body is a colon lambda
// (tests/errors/colon_lambda_single_line.scala). A single case after the colon is a lambda of it
// (`singleCaseMatch`), its body indented on the lines after it too; the body on the colon's line ends
// with it (tests/errors/colon_lambda_end.scala), a selection on the next line continuing the call.
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
  val e = List(1, 2).map: case x => x + 1
  val f = List(1, 2, 3).collect: case n if n > 1 => n * 10
  val g = List((1, "a"), (2, "b")).map: case (n, s) => s * n
  println((e, f, g))
  val three = List(1, 2, 3)
    .collect: case x =>
      val y = x + 1
      (x, y)
    .toMap
  val two = List(1, 2, 3)
    .collect: x => (x, x * 2)
    .toMap
  println((three, two))
