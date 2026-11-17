package wos

// Statements naming another file's objects: one removed (its class runs no initialiser to the reference), one
// evaluated (an inherited body), the result of an ascribed block (removed, its statements kept), a discarded
// value in a function literal typed Unit (removed).
object Unit2:
  inline def inlined(): Unit = { Removed2; println("  unit 2: in the inline block") }
  def run(): Unit =
    println("unit 2")
    Removed2
    Kept2
    ({ println("  unit 2: in the ascribed block"); Ascribed2 }: Any)
    val f: () => Unit = () => Removed2
    f()
    inlined()
