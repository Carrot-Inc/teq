// A plain inline given tied to its own by-name clause and found by `summonInline` inside an inline method: its
// call is kept for later, and the class its expansion makes then reads the knot's lazy local.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show:
  def show(n: Int): String
object Show:
  inline given sub(using s: => Show): Show = new Show:
    def show(n: Int): String = if n > 3 then "deep" else "sub:" + s.show(n + 1)
inline def get: Show = scala.compiletime.summonInline[Show]
@main def run(): Unit =
  println(get.show(1))
