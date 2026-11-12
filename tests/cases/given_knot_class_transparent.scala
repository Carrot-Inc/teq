// A given tied to its own by-name clause, transparent and inline, whose expansion makes a class that reads the
// knot's lazy local in a method.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  transparent inline given sub(using s: => Show): Show = new Show:
    def show(n: Int): String = if n > 3 then "deep" else "sub:" + s.show(n + 1)
@main def run(): Unit =
  println(summon[Show].show(1))
