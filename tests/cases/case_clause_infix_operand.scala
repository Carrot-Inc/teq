// An infix pattern's right operand on the line after its operator, inside a case (dotty's `InCase`).
def f(x: Any): Int = x match
  case 1 ::
      Nil => 1
  case _ => 0
@main def run(): Unit = println(f(List(1)))
