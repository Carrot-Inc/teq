// A wildcard and a `given` import from a stable val of an object (`import D.api.*`, `import
// D.api.given`) bring the val's members and givens, as a named selector does.
class API:
  def n = 7
  given Int = 8
object D:
  val api = new API
@main def run(): Unit =
  locally {
    import D.api.*
    println(n)
  }
  locally {
    import D.api.given
    println(summon[Int])
  }
