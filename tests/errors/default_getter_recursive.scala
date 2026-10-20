// A default is its getter's, a member of its own whose result is inferred (`Desugar.addDefaultGetters`): one that
// calls its own method leaving the parameter to it reads the getter while completing it, a cycle scalac reports as
// E044 (`Overloaded or recursive method f$default$1 needs return type`).
// expect: error: recursive use of f$default$1 needs an explicit result type
object G:
  def f(x: Int = f()): Int = x
@main def run(): Unit = println(G.f(1))
