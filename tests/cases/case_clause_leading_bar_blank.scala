// A leading `|` after a blank line inside a case: no separator in `InCase`, where outside one the blank line
// would end the statement (`isLeadingInfixOperator`'s `pastBlankLine`).
def f(x: Any): Int = x match
  case 1

      | 2 => 1
  case _ => 0
@main def run(): Unit = println(f(1))
