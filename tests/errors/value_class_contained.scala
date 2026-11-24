// scalac 3.8.4 refuses a value class that is not static (E075, `Checking.scala` 871-872): a local class, a class's
// member, and an object's member where the object is not static.
// expect: 10:9: error: Value classes may not be a member of another class
// expect: 12:11: error: Value classes may not be a member of another class
// expect: 18:9: error: Value classes may not be a local class
// expect: 19:14: error: Value classes may not be a local class
// expect: 21:11: error: Value classes may not be a member of another class
// expect: 6 errors found
class Holder:
  class Member(val n: Int) extends AnyVal
  object Inner:
    class InObject(val n: Int) extends AnyVal
object Placed:
  class Nested(val n: Int) extends AnyVal
  object Deeper:
    class Deepest(val n: Int) extends AnyVal
def block(): Int =
  class Local(val n: Int) extends AnyVal
  case class LocalCase(n: Int) extends AnyVal
  object LocalObject:
    class InLocalObject(val n: Int) extends AnyVal
  Local(1).n + LocalCase(2).n
// A given with no parameters is a module, as an object is (accepted); one with parameters is a class (refused).
trait Base
class Cfg
given G: Base with
  class InGiven(val n: Int) extends AnyVal
given H(using c: Cfg): Base with
  class InParamGiven(val n: Int) extends AnyVal
// expect: 29:9: error: Value classes may not be a member of another class
// An object nested in a given with no parameters, named or anonymous, is static as one nested in an object is
// (accepted), a companion there too.
given GO: Base with
  object In:
    class InGivenObject(val n: Int) extends AnyVal
  case class CaseInGiven(n: Int)
  object CaseInGiven:
    class InCompanion(val n: Int) extends AnyVal
given Base with
  object InAnonymous:
    class InAnonymousGiven(val n: Int) extends AnyVal
