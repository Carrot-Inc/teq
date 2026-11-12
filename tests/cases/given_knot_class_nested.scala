// The knot's lazy local read in a class inside the class an inline given's expansion makes.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  transparent inline given sub(using s: => Show): Show = new Show:
    def show(n: Int): String =
      val inner = new Show:
        def show(m: Int): String = s.show(m + 1)
      if n > 3 then "deep" else "sub:" + inner.show(n)
@main def run(): Unit =
  println(summon[Show].show(1))
