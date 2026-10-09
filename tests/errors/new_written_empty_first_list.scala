// expect: 9:34: error: no given instance of type Ctx was found for parameter ctx
// A written `()` before the lists of a constructor whose first clause is a using clause is an
// argument list, so the using clause is resolved in its place, as scalac does (`No given instance
// of type Ctx was found for parameter ctx of constructor Node`); `new Node(using c)(..)` and
// `new Node(..)` with a given in scope are `tests/cases/new_leading_using_clause.scala`.
class Ctx(val level: Int)
class Node(using val ctx: Ctx)(label: String):
  def show = s"${ctx.level} $label"
@main def Main(): Unit = println(new Node()(using Ctx(1))("a").show)
