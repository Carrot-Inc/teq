// A trait of using clauses alone applied with a named list that leaves a parameter out before a written one, its empty
// clause not written: E100 once the parameter's search succeeds, as scalac refuses it; with no given for it, the search's
// own E172. By a parent and by an anonymous class.
// expect: 15:19: error: constructor T in trait T must be called with () argument
// expect: 16:27: error: constructor T in trait T must be called with () argument
// expect: 18:19: error: No given instance of type B was found
// expect: 19:27: error: No given instance of type B was found
class A(val v: Int)
class B(val v: Int)
class C(val v: Int)
trait T(using a: A, b: B, c: C):
  def value: String = s"${a.v} ${b.v} ${c.v}"
object WithGiven:
  given B = B(200)
  class S extends T(using a = A(1), c = C(3))
  def anon: String = (new T(using a = A(1), c = C(3)) {}).value
object NoGiven:
  class S extends T(using a = A(1), c = C(3))
  def anon: String = (new T(using a = A(1), c = C(3)) {}).value
