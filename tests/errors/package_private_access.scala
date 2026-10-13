// A private definition of a package, or of its package object (every top-level term but an object), is
// `private[p]` of the package (dotty's `Namer`: "widen private to private[p]"), and neither it nor a
// written `private[p]` one is accessible through the package from outside it (`isAccessibleFrom`, E173);
// a wildcard import brings no inaccessible member (`Not found`). Master accepted every selection here.
package jam {
  trait Dsl { def v: Int = 1 }
  private object internal extends Dsl
  private class K
  private def pd: Int = 2
  private[jam] def qd: Int = 3
  object inside { val ok = internal.v + pd + qd + (new K).hashCode * 0 }
}
object test {
  val a = jam.internal.v
  val b = jam.internal
  val c = new jam.K
  val d = jam.pd
  val q = jam.qd
  import jam.*
  val e = pd
  val f = inside.ok
}
// expect: 14:11: error: object internal cannot be accessed as a member of jam.type from object test.
// expect: private[jam] object internal can only be accessed from package jam.
// expect: 15:11: error: object internal cannot be accessed as a member of jam.type from object test.
// expect: 16:15: error: class K cannot be accessed as a member of jam.type from object test.
// expect: private[jam] class K can only be accessed from package jam.
// expect: 17:11: error: method pd in package jam cannot be accessed as a member of jam.type from object test.
// expect: private[jam] method pd can only be accessed from package jam.
// expect: 18:11: error: method qd in package jam cannot be accessed as a member of jam.type from object test.
// expect: 20:11: error: not found: pd
