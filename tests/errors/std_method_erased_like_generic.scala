// expect: std_method_erased_like_generic.scala:7:7: error: class Graded inherits compareTo as (other: Graded): Int and as [T](other: T): Int
// A class that takes, beside `Comparable`'s `compareTo`, a trait's `compareTo` with a type parameter of its own:
// the two erase to the same parameter and result, and scalac 3.8.4 reports E120, a name clash between inherited
// members, at the class.
trait Extra { def compareTo[T](other: T): Int = 7 }

class Graded(val n: Int) extends Extra with Comparable[Graded] { def compareTo(other: Graded): Int = n - other.n }
