// expect: value class can only have one non `erased` parameter
// expect: A value class parameter may not be a var
// expect: Value classes may not define non-parameter field
// expect: Value classes may not define an inner class
// expect: Value class needs one val parameter
// expect: Value classes may not be abstract
// expect: only a class can extend AnyVal
// expect: type mismatch: found String, required AnyVal
class A(val x: Int, val y: Int) extends AnyVal
class B(var x: Int) extends AnyVal
class C(val x: Int) extends AnyVal:
  val y = 1
class D(val x: Int) extends AnyVal:
  class Inner
class G() extends AnyVal
abstract class K(val x: Int) extends AnyVal
trait T extends AnyVal
object M:
  val v: AnyVal = "s"
// expect: value class cannot wrap itself
// expect: value class may not wrap another user-defined value class
class Self(val v: Self) extends AnyVal
class Inner1(val x: Int) extends AnyVal
class Outer1(val y: Inner1) extends AnyVal
