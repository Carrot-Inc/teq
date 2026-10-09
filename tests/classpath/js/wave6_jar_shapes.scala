// jars: fixtures
// targets: js interp jvm
// Two shapes that failed from a jar only (in scalajs-react's jars): an extension whose
// first clause is a using clause (`extension (using q: Quotes)(self: q.reflect.Term)` in microlibs),
// called from source (from a jar's body in wave6_using_ext_body.scala), and members an object of a jar inherits from its
// trait, exported by name and renamed next to a wildcard (`export HtmlTags.{main => mainTag, *}`).
// The expectation is scalac 3.8.4's, run over tests/tasty/src/wave6.scala and this file.
import fix.wave6.*

object Prelude:
  export W6Tags.{main => mainTag, *}
object Named:
  export W6Tags.div

@main def main(): Unit =
  given c: W6Ctx = new W6Ctx
  val t = c.term("a")
  println(t.tagged[Int])
  println(Prelude.mainTag)
  println(Prelude.div)
  println(Named.div)
