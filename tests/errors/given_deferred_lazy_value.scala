// A deferred given without parameters is a lazy value, overridden as one (dotty's `RefChecks`):
// by a stable value alone, so an abstract given def re-declaring it is refused, and the class below
// it is refused for leaving that def unimplemented (nothing implements the deferred one behind it);
// an eager val must be declared lazy.
// expect: 12:36: error: method x needs to be a stable, immutable value to override given x in trait T
// expect: 13:7: error: class C needs to be abstract, since override given def x: Int in trait U is not defined
// expect: 14:45: error: method x needs to be a stable, immutable value to override given x in trait T
// expect: 15:27: error: given x needs `override` modifier to override given x in trait T
// expect: 16:34: error: value x must be declared lazy to override given x in trait T
import scala.compiletime.deferred
trait T { given x: Int = deferred }
trait U extends T { override given x: Int }
class C(using Int) extends U
abstract class B extends T { override given x: Int }
class D extends B { given x: Int = 4 }
trait V extends T { override val x: Int = 23 }
