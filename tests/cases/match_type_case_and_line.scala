// A match type's intersection pattern on one line, the control of `match_type_case_trailing_and`.
type T[X] = X match
  case Int & Matchable => Int
  case _ => String
@main def run(): Unit = { val a: T[Int] = 1; println(a) }
