// expect: 22:16: error: Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.
// expect: 25:17: error: Case object definitions are not allowed in inline methods or quoted code. Use a normal object instead.
// expect: 28:19: error: Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.
// expect: 31:16: error: Implementation restriction: nested inline methods are not supported
// expect: 35:18: error: Implementation restriction: nested inline methods are not supported
// expect: 40:30: error: Super call not allowed in inlineable method viaSuper
// expect: 42:29: error: Super call not allowed in inlineable method local
// expect: 46:18: error: Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.
// expect: 8 errors found
// What scalac 3.8.4 refuses in an inline method's body, at the definition, called or not, the
// eight errors it reports here: a case class, a case object or an enum case with parameters in a
// method that is a member
// (`PrepareInlineable.makeInlineable`, E162, which a local inline method is spared), a nested
// inline method ("nested inline methods are not supported", local or not) and a `super` of the
// method's own class (E082, `typedSuper`). Each refused definition is a failed one: its call is
// a plain call of its declared type, expanded no more.
class Base:
  def n: Int = 1

object Lib:
  inline def caseClass: Int =
    case class C(n: Int)
    C(1).n
  inline def uncalledCaseClass: Int =
    case object K
    1
  inline def enumClassCase: Int =
    enum E { case A(n: Int) }
    E.A(1).ordinal
  inline def nested: Int =
    inline def g: Int = 1
    g
  def outer: Int =
    inline def local: Int =
      inline def g: Int = 2
      g
    local

class Derived extends Base:
  inline def viaSuper: Int = super.n
  def m: Int =
    inline def local: Int = super.n
    local
  inline def nestedCase: Int =
    class Holder:
      case class Inner(v: Int)
      def get = Inner(3).v
    new Holder().get

@main def run(): Unit =
  println(Lib.caseClass + Lib.enumClassCase + Lib.nested + Lib.outer)
  println(new Derived().viaSuper + new Derived().m + new Derived().nestedCase)
