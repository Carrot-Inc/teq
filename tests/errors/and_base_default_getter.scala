// A default getter of a member whose owner the receiver's base gives at two instances is seen from them as the member
// is (`asSeenFrom` of `q.m$default$n` on the receiver, `argForParam`'s `AndOrType` arm and `AsSeenFromMap.derivedSelect`'s
// range, its result at a result's variance): through `Both`'s `Box[Int] & Box[String]` the getter of `b: List[A]` is a
// `List[Int & String]`, which the parameter's `List[Int | String]` takes; through `IB | SB` it is a
// `List[Int | String]`, which `List[Int & String]` does not take. scalac 3.8.4: one error, at line 14 ("Found:
// List[Int | String] @uncheckedVariance, Required: List[Int & String]"; dotty's neg/i21535 the same shape).
// expect: 14:26: error: Found:    List[Int | String]
// expect: 1 error found
trait Box[A]:
  def put(a: A, b: List[A] = Nil): Int
trait IB extends Box[Int]
trait SB extends Box[String]
trait Both extends IB with SB
def h(x: IB | SB): Int = x.put(???)
def f(x: Both): Int = x.put(1) + x.put("s")
// TypeMismatch divergence owner (coordinator ruling, 2026-11-05): Applications.defaultArgument (the found type's
// `@uncheckedVariance`, which scalac shows and teq does not).
// expect: Required: List[Int & String]
