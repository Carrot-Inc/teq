package wos

// Statements naming another file's objects: one removed (its class runs no initialiser to the reference), one
// evaluated (an inherited body), the result of an ascribed block (removed, its statements kept), a discarded
// value in a function literal typed Unit (removed).
object Unit1:
  inline def inlined(): Unit = { Removed1; println("  unit 1: in the inline block") }
  def run(): Unit =
    println("unit 1")
    Removed1
    Kept1
    ({ println("  unit 1: in the ascribed block"); Ascribed1 }: Any)
    val f: () => Unit = () => Removed1
    f()
    inlined()
