// expect: type mismatch: found Int, required T
enum Expr[+T]:
  case I1() extends Expr[Int]
  case I2() extends Expr[Int]
  case B() extends Expr[Boolean]
import Expr.*

def bar[T](e: Expr[T]): T =
  e match
    case I1() | B() => 42
    case I2() => 0
