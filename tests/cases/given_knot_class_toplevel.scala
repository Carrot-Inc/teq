// The search that ties the knot stands in a top-level `val` and in a parameter's default; the plain inline
// given's class reads the lazy local.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  inline given sub(using s: => Show): Show = new Show:
    def show(n: Int): String = if n > 3 then "deep" else "sub:" + s.show(n + 1)
val shown: Show = summon[Show]
class Use(val s: Show = summon[Show])
@main def run(): Unit =
  println(shown.show(1))
  println(Use().s.show(2))
