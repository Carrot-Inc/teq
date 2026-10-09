// A transparent inline method passes on the type of its expansion: a constant body's literal type,
// and the type a body ending in an ascription was given, which the constant's evaluation does not
// replace, whether it widens the literal (`(1: Any)`), names its class (`(1: Int)`, through an alias
// too) or its literal type (`(1: 1)`). scalac 3.8.4 prints "any int int" and "int int int one one three".
type I = Int
transparent inline def widened: Any = (1: Any)
transparent inline def narrowed: Any = 1
transparent inline def folded: Int = 1 + 2
transparent inline def ascribedInt: Any = (1: Int)
transparent inline def ascribedAlias: Any = (1: I)
transparent inline def ascribedLit: Any = (1: 1)
transparent inline def inBlock: Any = { val u = (); (2: Int) }
def pick(x: Int): String = "int"
def pick(x: Any): String = "any"
def choose(x: Singleton): String = "singleton"
def choose(x: Int): String = "int"
def one(x: 1): String = "one"
def three(x: 3): String = "three"
@main def main(): Unit =
  println(List(pick(widened), pick(narrowed), pick(folded)).mkString(" "))
  println(List(choose(ascribedInt), choose(ascribedAlias), choose(inBlock), one(ascribedLit), one(narrowed), three(folded)).mkString(" "))
