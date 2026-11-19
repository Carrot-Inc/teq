// A member marked by `@uncheckedOverride` is marked as an override (`RefChecks.isMarkedOverride`) for every check
// that reads the mark: it may not override a mutable variable, and it is an accidental override of a concrete member
// it shares with no third one. An annotation of another class named so marks nothing.
import scala.annotation.unchecked.uncheckedOverride
class Variable { var v: Int = 1 }
class OverVariable extends Variable { @uncheckedOverride def v: Int = 2 }
trait P { def g = "P" }
trait Q { def g = "Q" }
trait S extends P { @uncheckedOverride def g = "S" }
class Accidental extends Q with S
object Other:
  class uncheckedOverride extends scala.annotation.StaticAnnotation
  class Base { def m: Int = 1 }
  class Unmarked extends Base { @uncheckedOverride def m: Int = 2 }
// expect: 6:62: error: error overriding variable v in class Variable of type Int;
// expect: 10:7: error: g in trait S cannot override the concrete g in trait Q without a member that both override
// expect: 14:56: error: method m needs `override` modifier to override method m in class Base
// expect: 3 errors found
