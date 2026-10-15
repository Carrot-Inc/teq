// A leading `|` on the next line of a case (`isLeadingInfixOperator`), a control of the case region.
def f(x: Any): Int = x match
  case 1
      | 2 => 1
  case _ => 0
@main def run(): Unit = println(f(1))
