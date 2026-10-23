// `ApproximatingTypeMap.derivedAppliedType` at a positive variance over an alias application whose argument escapes
// (Types 6765-6815 at 3.8.4): the application is rebuilt with its range as bounds and reduced (`Twice[?]` is
// `Box[Box[?]]`), falling back to distributing the range only for an unreducible wildcard application. So scalac
// 3.8.4 approximates the closure's result to `Out[Box[Box[?]]]`, which the result `Out[Twice[x.T]]` does not conform
// to, E007. teq's avoidance returns the canonical map's own result, `Out[Box[? <: Box[?]]]`, and accepts it; without
// the alias both compilers give that type and accept. source-version-3's `derivedAppliedType` port (the pass over
// fix-semantic's c02b18202, item 5); this case goes live at the rebase onto its landing. scalac's line.
// expect: 16:34: error: type mismatch: found Out[Twice[x.T]], required Out[Box[Box[_]]]
package twice
class C { type T }
class Box[A]
class Out[+A]
object TwiceProbe:
  type Twice[A] = Box[Box[A]]
  def map[A, B](a: A)(f: A => B): B = f(a)
  val outTwice = map(new C)(x => (??? : Out[Twice[x.T]]))
