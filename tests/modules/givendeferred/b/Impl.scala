package gdb

import gda.*

object Site:
  given fresh(using d: DummyImplicit): String = { Counter.made += 1; "site" + Counter.made }
  class Plain extends Named
  object Obj extends Named

class Sorted(using o: Ordering[Int]) extends Ordered[Int]

class WithShown extends Shown:
  override given shown(using n: Int): String = "n=" + n

class IntBox(using n: Int) extends Boxed[Int]

class QualifiedImpl(using n: Int) extends Qualified
