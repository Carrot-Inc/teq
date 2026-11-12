// The knot's lazy local read in a function literal that stands for the trait (a class too), in the expansion of
// a transparent inline given tied to its own by-name clause.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  transparent inline given sub(using s: => Show): Show = n => if n > 3 then "deep" else "sub:" + s.show(n + 1)
@main def run(): Unit =
  println(summon[Show].show(1))
