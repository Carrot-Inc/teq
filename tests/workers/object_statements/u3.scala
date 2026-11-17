package wos

// Statements naming another file's objects: one removed (its class runs no initialiser to the reference), one
// evaluated (an inherited body), the result of an ascribed block (removed, its statements kept), a discarded
// value in a function literal typed Unit (removed).
object Unit3:
  inline def inlined(): Unit = { Removed3; println("  unit 3: in the inline block") }
  def run(): Unit =
    println("unit 3")
    Removed3
    Kept3
    ({ println("  unit 3: in the ascribed block"); Ascribed3 }: Any)
    val f: () => Unit = () => Removed3
    f()
    inlined()
