// expect: std_method_erased_like_inherited.scala:6:7: error: class Second inherits compareTo as (b: Second): Int and as (o: Any): Int
// A class that takes `compareTo(o: Any)` from its superclass beside `Comparable`'s, the two erasing to the same
// parameter and result: scalac 3.8.4 reports E120, a name clash between inherited members, at the class.
class First { def compareTo(o: Any): Int = 0 }

class Second extends First with Comparable[Second] { def compareTo(b: Second): Int = 0 }
