// expect: exports_cycle_alone.scala:9:10: error: cyclic export of B, which exports A
// Two objects that export each other, nothing importing either: whichever table is demanded
// first (the order of the completions, TEQ_DEMAND_ORDER=reversed among them), the cycle is
// reported once, at the export of the object that comes first in the file. scalac rejects
// the program too, as E120 (the members exported back conflict with the objects' own).
package cyc

object A:
  export B.*
  def a: Int = 1

object B:
  export A.*
  def b: Int = 2
