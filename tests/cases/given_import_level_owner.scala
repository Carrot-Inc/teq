// An import starts a level of its own under another owner than the scope around it (`ContextualImplicits.level`):
// a val's right-hand side, a lambda and a local def are owners, so an import there is nearer than the method's
// given; two imports of one owner stand at one level, the inner one hiding a same-named given of the outer one
// unless the outer one's precedence beats it (a named import over a wildcard, `combineEligibles`).
object Values:
  given Int = 7
  given s: String = "values"
object Other:
  given s: String = "other"
def show(f: () => Int): Int = f()
@main def run(): Unit =
  given Int = 9
  val inVal: Int = { import Values.given; summon[Int] }
  println(inVal)
  println(show(() => { import Values.given; summon[Int] }))
  def local: Int = { import Values.given; summon[Int] }
  println(local)
  locally {
    import Values.given
    locally {
      import Other.given
      println(summon[String])
    }
  }
  locally {
    import Other.given
    locally {
      import Values.{given String}
      println(summon[String])
    }
  }
