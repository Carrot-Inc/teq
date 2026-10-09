enum Expr[+T]:
  case I1() extends Expr[Int]
  case I2() extends Expr[Int]
  case B() extends Expr[Boolean]
import Expr.*

def foo[T](e: Expr[T]): T =
  e match
    case I1() | I2() => 42
    case B() => true

@main def run(): Unit =
  println(foo(I2()))
  println(foo(B()))
