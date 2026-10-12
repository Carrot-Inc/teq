// expect: 24:16: error: object Val in class Enumeration does not take parameters
// expect: 25:18: error: object Val in class Enumeration does not take parameters
// expect: 28:13: error: Outer is not a valid class prefix, since it is not an immutable path
// expect: 43:13: error: object D in package p does not take parameters
// expect: 4 errors found
// absent: 36:
// The constructor proxy `apply` dotty adds to a class's explicit companion has the constructor's access,
// protected for a protected class, and the class's qualifier (NamerOps.addConstructorApplies): a call
// that cannot reach it finds no `apply`. Through a companion read on an instance, the proxy's `new`
// needs a path as its class prefix (E083); a case class's own `apply` makes its own. A protected
// class's proxy is protected only where its companion is not.
class Enumeration:
  protected class Val(i: Int):
    def this() = this(42)
  object Val

class Outer:
  class A(i: Int)
  object A
  case class B(i: Int)
  object B

class Test extends Enumeration:
  val Hearts = Val(27)
  val Diamonds = Val()

object Use:
  def h() = new Outer().A(1)
  def k() = new Outer().B(2)

class Companioned:
  protected class Val(i: Int)
  protected object Val

class Uses extends Companioned:
  val Hearts = Val(27)

package p:
  private[p] class D(i: Int)
  object D

package q:
  def g() = p.D(42)
