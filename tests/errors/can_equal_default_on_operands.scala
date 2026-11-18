// Without strictEquality an operand with its own CanEqual instance compares only by evidence, though a bound
// or a part of it has none: the defaults are decided on the operands as written.
class A derives CanEqual
trait B
class C
class D
object O:
  opaque type T <: D = D
  object T:
    given CanEqual[T, T] = CanEqual.derived
trait Holder:
  type M <: A & B
  def m(x: M, y: C): Boolean = x == y
object Main:
  def l(x: A & B, y: C): Boolean = x == y
  def r(x: C, y: A & B): Boolean = x != y
  def o(x: O.T, y: C): Boolean = x == y
  def same(x: A & B, y: A): Boolean = x == y
  def bound(x: O.T, y: D): Boolean = x == y

// expect: can_equal_default_on_operands.scala:13:32: error: values of types A & B and C cannot be compared with == or !=
// expect: can_equal_default_on_operands.scala:15:36: error: values of types A & B and C cannot be compared with == or !=
// expect: can_equal_default_on_operands.scala:16:36: error: values of types C and A & B cannot be compared with == or !=
// expect: can_equal_default_on_operands.scala:17:34: error: values of types T and C cannot be compared with == or !=
// expect: 4 errors found
