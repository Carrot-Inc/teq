// Under `relaxedLambdaSyntax` a lambda on the colon's line ends with the line (dotty's
// `SingleLineLambda` region, closed by the `ENDlambda` its scanner inserts there): a token after
// the body on that line is an error, as scalac 3.8.4 reports it (E040), the single-case form too,
// and an operand cannot come from the next line (E018).
import language.experimental.relaxedLambdaSyntax
def f(g: Int => Int): Int = g(1)
def run() =
  val b = f: x => x + 1; val c = 4
  val d = { f: x => x + 1 }
  val e = List(1, 2).map: case x => x + 1; println(e)
  val g = f: x => x +
    3
// expect: 8:24: error: end of single-line lambda expected, but ';' found
// expect: 9:27: error: end of single-line lambda expected, but '}' found
// expect: 10:42: error: end of single-line lambda expected, but ';' found
// expect: 11:22: error: expected an expression, found end of single-line lambda
