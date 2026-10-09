// A deferred given is overridden as a concrete member is, with `override` (dotty's `RefChecks`,
// 533-546), a parameterized one too, and the implementation a class was given for it is final, to a
// subclass's member and to a trait the subclass mixes in.
// expect: 11:27: error: given x needs `override` modifier to override given x in trait T
// expect: 12:27: error: given list needs `override` modifier to override given list in trait U
// expect: 14:46: error: given x cannot override final member given x in class D
// expect: 16:7: error: given x in trait V cannot override final member given x in class D
import scala.compiletime.deferred
trait T { given x: Int = deferred }
trait U { given list[A]: List[A] = deferred }
class C extends T { given x: Int = 9 }
class P extends U { given list[A]: List[A] = Nil }
class D(using Int) extends T
class E extends D(using 11) { override given x: Int = 22 }
trait V extends T { override given x: Int = 99 }
class F extends D(using 12) with V
