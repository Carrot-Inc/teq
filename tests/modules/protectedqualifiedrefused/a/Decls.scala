package lib.inner

// Protected members qualified by `lib`, read through a self type from outside lib, over teq's products: no enclosing
// class can hold the accessor such a reference needs, so each is refused as scalac refuses it (ProtectedAccessors'
// `ifNoHost`), as the same references are refused where the members are in the program's sources.
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
