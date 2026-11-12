// As macro_suspended_after_std_demand_retried, with a macro that keeps a flag in its own object: the file
// that waited is compiled again, the object is made again with it, and the macro calls the standard library
// method a second time, which the first compilation's run had demanded.
package rounds

object Reads:
  val counted: Int = Counting.pair
  val fixed: Int = Fixing.level
