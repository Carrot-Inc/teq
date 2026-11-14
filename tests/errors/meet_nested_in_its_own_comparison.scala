// expect: 19:7: error: recursive use of at needs an explicit result type
// expect: 1 error found
// absent: Conflicting definitions
// absent: Ambiguous overload
// The result of `Writes.at(n: Int)` is inferred from a body that selects `at` on `Reads & Writes`, the
// intersection whose overloads `Early.first` is already comparing: the comparison nested there reads that
// signature in progress. scalac reports the cycle (E044) and nothing else; the set of alternatives made
// by the nested comparison is its caller's alone, so the forwarders `export x.at` makes are those of the
// whole comparison, and no second `at(n: Int)` clashes with the first.
object Early:
  val first = { val x: Reads & Writes = null; x.at("k") }

trait Reads:
  def at(key: String): String
  def at(n: Int): Int

trait Writes:
  def at(row: Int, col: Int, value: Int): Int
  def at(n: Int) = { val x: Reads & Writes = null; x.at(1, 2, 3) }

class Both(val x: Reads & Writes):
  export x.at

object Late:
  def use(b: Both) = (b.at("z"), b.at(1, 2, 3), b.at(5))
