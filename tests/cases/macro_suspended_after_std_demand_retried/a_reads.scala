// As macro_suspended_after_std_demand, but the file that waited is the only one to run the macro again: its
// own next compilation, one round later, calls the standard library method the first one demanded.
package rounds

object Reads:
  val counted: Int = Counting.pair
  val fixed: Int = Fixing.level
