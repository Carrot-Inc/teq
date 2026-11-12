// An enum case whose companion is written with an `apply` of its own: the application has two
// alternatives, the case's creation and the written `apply`, so no callee is read from it and the lambda's
// parameter has no type, as for a case class with such a companion (scalac 3.8.4: E081, missing parameter
// type, at the placeholder).
// expect: 15:11: error: Ambiguous overload. The overloaded alternatives of method apply in object Wrapped with types
// expect: 15:24: error: missing parameter type
enum Node:
  case Wrapped(inner: Int)
  case Plain
object Node:
  object Wrapped:
    def apply(text: String): Node = new Wrapped(text.length)

@main def run(): Unit =
  val f = Node.Wrapped(_)
  println(f(1))
