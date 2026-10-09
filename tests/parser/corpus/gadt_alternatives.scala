// GADT constraints from alternatives that agree, and from type-test patterns.
enum Expr[+T]:
  case I1() extends Expr[Int]
  case I2() extends Expr[Int]
  case B() extends Expr[Boolean]
import Expr.*

class Invariant[T](val value: T)

def foo[T](e: Expr[T]): T =
  e match
    case I1() | I2() => 42
    case B() => true

def get[T](i: Invariant[T]): T = i match
  case _: Invariant[Int] => i.value + 1
  case _ => i.value

@main def run(): Unit =
  println(foo(I2()))
  println(foo(B()))
  println(get(new Invariant(1)))
