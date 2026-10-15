// expect: 16:14: error: error overriding method C in trait AA of type Int => AA.this.C: object C has incompatible type
// expect: 18:10: error: error overriding method D in trait AA of type Int => AA.this.D: object D has incompatible type
// expect: 2 errors found
// An object's module val overrides what its class inherits of its name, checked as `RefChecks`
// checks the module's symbol: the companion `Desugar.classDef` makes for a case class
// (dotty's tests/neg/t3137.scala) and a written one. An object implementing an abstract val
// of a type it has is no error. scalac places the written object's error at `object` (18:3), teq's
// definitions at their names.
trait AA:
  type C
  def C: Int => C
  type D
  def D: Int => D
  val E: Any
class BB extends AA:
  case class C(v: Int)
  class D
  object D
  object E
