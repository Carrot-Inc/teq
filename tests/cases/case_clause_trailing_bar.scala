// A case's alternatives continued past a trailing `|`: the pattern and guard of a case are dotty's `InCase`
// region (`Parsers.caseClause`), where a line break separates nothing (`Scanners.handleNewLine`).
enum S:
  case A, B, C
  case D(n: Int)
def f(s: S): Int = s match
  case S.A | S.B |
      S.C | S.D(_) => 1
@main def run(): Unit = println(f(S.D(3)))
