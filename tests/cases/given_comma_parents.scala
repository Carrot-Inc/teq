// A given's parents after a comma, under 3.6's syntax, with its body after a colon (dotty's
// `Parsers.givenDef`: `implemented`'s `moreConstrApps`), anonymous and named: the given's object extends them all.
// scalac 3.8.4 prints `ab` and `abb`.
trait A { def a = "a" }
trait B { def b = "b" }
object Anonymous:
  given A, B:
    def both = a + b
object Named:
  given ab: A, B:
    def both = a + b
@main def run(): Unit =
  locally:
    import Anonymous.given
    println(summon[A].a + summon[B].b)
  println(Named.ab.both + summon[B](using Named.ab).b)
