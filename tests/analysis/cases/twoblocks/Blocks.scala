// Two package blocks, each with top-level methods of its own `<file>$package`.
package tba:
  def inc(x: Int): Int = x + 1
  val base: Int = 10
  class Counter:
    def next: Int = inc(base)

package tbb:
  def dec(y: Int): Int = y - 1
  class Other:
    def prev: Int = dec(tba.inc(1))
