// A given with its parameters before the colon is the syntax before 3.6, which turns 3.6's off
// for it (dotty's `Parsers.givenDef`, `hasEmbeddedColon`): its body follows `with`, not a colon
// (scalac: "'with' expected, but ':' found").
// expect: 7:35: error: 'with' expected, but ':' found
trait Parent(val n: Int)
@main def run() =
  given t(using n: Int): Parent(n):
    def extra = n + 1
  given Int = 3
  println(t.extra)
