// expect: exports_cycle_alone.scala:9:12: error: Conflicting definitions:
// Two objects that export each other, nothing importing either: the first one's forwarders,
// made first whatever the order of the completions (TEQ_DEMAND_ORDER=reversed among them),
// read the second one's, which forward the first one's own member back: E120 at the first
// one's export, as scalac reports it.
package cyc

object A:
  export B.*
  def a: Int = 1

object B:
  export A.*
  def b: Int = 2
