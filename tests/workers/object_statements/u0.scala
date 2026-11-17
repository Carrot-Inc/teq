package wos

// Statements naming another file's objects: one removed (its class runs no initialiser to the reference), one
// evaluated (an inherited body), the result of an ascribed block (removed, its statements kept), a discarded
// value in a function literal typed Unit (removed).
object Unit0:
  inline def inlined(): Unit = { Removed0; println("  unit 0: in the inline block") }
  def run(): Unit =
    println("unit 0")
    Removed0
    Kept0
    ({ println("  unit 0: in the ascribed block"); Ascribed0 }: Any)
    val f: () => Unit = () => Removed0
    f()
    inlined()
