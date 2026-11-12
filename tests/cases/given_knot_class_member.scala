// The search that ties the knot stands in a member's initializer; the inline given's class reads the lazy local.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  transparent inline given sub(using s: => Show): Show = new Show:
    def show(n: Int): String = if n > 3 then "deep" else "sub:" + s.show(n + 1)
object Use:
  val shown: Show = summon[Show]
@main def run(): Unit =
  println(Use.shown.show(1))
