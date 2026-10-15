// A match type's case pattern continued after a trailing `&` and a blank line: `typeCaseClause` parses its
// pattern inside `InCase` too.
type T[X] = X match
  case Int &

    Matchable => Int
  case _ => String
@main def run(): Unit = { val a: T[Int] = 1; println(a) }
