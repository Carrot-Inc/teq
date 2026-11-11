// A bounded wildcard is checked wherever its application is written (`PostTyper`'s `AppliedTypeTree` case,
// `Checking.checkAppliedType`): nested in another application, in a method's parameter and result, a type
// parameter's bound, a val's and a var's type, a refinement, a using clause, an opaque alias, a self type, a type
// test, an ascription and another wildcard's bound. scalac 3.8.4 reports every one (E057, `Type argument  >: X does
// not overlap with upper bound Int`, the same text at each) at the wildcard, one column before teq's.
class C[T <: Int]
type L = List[C[? >: String]]
type M = Map[String, C[? >: CharSequence]]
def f(c: C[? >: String]): C[? >: Boolean] = ???
def g[U <: C[? >: String]]: Int = 1
object O:
  val v: C[? >: String] = null
  var u: Option[C[? >: String]] = None
  opaque type Op = C[? >: String]
type R = { def m: C[? >: String] }
def h(using c: C[? >: String]): Int = 1
trait S:
  self: C[? >: String] =>
def t(x: Any): Boolean = x.isInstanceOf[C[? >: String]]
def a = (null: C[? >: String])
type W = List[? <: C[? >: String]]
// expect: 7:17: error: type argument ? >: String does not overlap with upper bound Int
// expect: 8:24: error: type argument ? >: CharSequence does not overlap with upper bound Int
// expect: 9:12: error: type argument ? >: String does not overlap with upper bound Int
// expect: 9:29: error: type argument ? >: Boolean does not overlap with upper bound Int
// expect: 10:14: error: type argument ? >: String does not overlap with upper bound Int
// expect: 12:12: error: type argument ? >: String does not overlap with upper bound Int
// expect: 13:19: error: type argument ? >: String does not overlap with upper bound Int
// expect: 14:22: error: type argument ? >: String does not overlap with upper bound Int
// expect: 15:21: error: type argument ? >: String does not overlap with upper bound Int
// expect: 16:18: error: type argument ? >: String does not overlap with upper bound Int
// expect: 18:11: error: type argument ? >: String does not overlap with upper bound Int
// expect: 19:43: error: type argument ? >: String does not overlap with upper bound Int
// expect: 20:18: error: type argument ? >: String does not overlap with upper bound Int
// expect: 21:22: error: type argument ? >: String does not overlap with upper bound Int
// expect: 14 errors found
