package rbna

// A refinement's by-name parameter: its method type states `=> Int`.
object ByName:
  def keep(f: AnyRef { def run(x: => Int): Int }) = f
