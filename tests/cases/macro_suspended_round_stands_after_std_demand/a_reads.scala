// A file waits for the macros of its run and is compiled again; in the round that stands, its first macro's
// run makes the first call of a standard library method whose body holds inline calls, and the program calls
// that method too.
package rounds

object Reads:
  val counted: Int = Counting.pair
  val fixed: Int = Fixing.level
