// A short `using` list written to the constructor of a class of using clauses alone: the reference's constructor has an
// empty clause after them, which teq's has not; there a parameter omitted before a written argument is E100 ("must be
// called with () argument") and an empty `()` written before the list has taken the clause whole, so those parameters
// are not searched and the refusal stays teq's own. The trailing ones are searched (tests/cases using_list_short_searched,
// the enum case of ParentCallAndEnum).
// expect: 17:23: error: missing argument for parameter a
// expect: 18:27: error: missing argument for parameter a
// expect: 20:20: error: missing argument for parameter a
// expect: 22:12: error: missing argument for parameter a
// expect: 23:12: error: missing argument for parameter a
// expect: 24:12: error: missing argument for parameter a
// expect: 24:28: error: missing argument for parameter a
class A(val s: String)
class B(val s: String)
given A = A("ga")
class K(using val a: A, val b: B)
class NoEmpty extends K(using b = B("wb"))
class EmptyBefore extends K()(using b = B("wb"))
enum E(using val a: A, val b: B):
  case One extends E(using b = B("wb"))
object Creations {
  val k1 = new K(using b = B("wb"))
  val k2 = new K()(using b = B("wb"))
  val k3 = new K(using b = new K(using b = B("in")).b)
}
