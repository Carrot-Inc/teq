// expect: std_method_erased_like_wrapped.scala:8:7: error: class Ranked inherits compareTo as (b: Ranked): Int and as (o: Any): Int
// A class that takes, beside `Comparable`'s `compareTo`, a trait's `compareTo` whose parameter is a value class
// over `Any`: the two erase to the same parameter and result, and scalac 3.8.4 reports E120, a name clash between
// inherited members, at the class.
package wrapped { class Any(val x: scala.Any) extends AnyVal }

trait Extra { def compareTo(o: wrapped.Any): Int = 7 }
class Ranked(val n: Int) extends Extra with Comparable[Ranked] { def compareTo(b: Ranked): Int = n - b.n }
