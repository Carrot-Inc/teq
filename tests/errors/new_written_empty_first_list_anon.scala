// expect: 7:34: error: No given instance of type Ctx was found for parameter ctx of constructor Node in class Node
// A written `()` before a leading using clause is an argument list for an anonymous class's
// parent as well (scalac: `No given instance of type Ctx was found for parameter ctx`).
class Ctx(val level: Int)
class Node(using val ctx: Ctx)(label: String):
  def show = s"${ctx.level} $label"
@main def Main(): Unit = println(new Node()(using Ctx(1))("a") {}.show)
