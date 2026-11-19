package lib.inner

// Protected members qualified by `lib`, declared in lib.inner and made by the compiler in other places: a deferred
// given's implementation in a class of another package, a case class's copy, an enum's member, an export's forwarder,
// a member a class of another package inherits, a method's default. Read from lib.other through teq's products, each
// stands inside its qualifier: no accessor is needed (dotty's ProtectedAccessors reads `accessBoundary`).
trait Q:
  protected[lib] given g: Int = scala.compiletime.deferred

case class K protected[lib] (x: Int)

enum E:
  case A
  protected[lib] def m: Int = 4

class H:
  protected[lib] def m: Int = 5

object O:
  private val h = new H
  export h.m

class B:
  protected[lib] def m: Int = 6

class D:
  protected[lib] def m(x: Int = 7): Int = x
