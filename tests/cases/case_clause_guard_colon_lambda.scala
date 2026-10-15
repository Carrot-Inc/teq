// A guard's colon lambda (`exists: x =>`, its body indented) inside the case region: its `=>` belongs to the lambda,
// not to the case, whose region goes on past a blank line to `&& true` and the case's own `=>` (dotty's parser ends
// `InCase` after the guard, `Parsers.caseClause`).
@main def run(): Unit =
  println(1 match
    case n if List(1).exists: x =>
      x > 0

    && true
    => n
    case _ => 0)
