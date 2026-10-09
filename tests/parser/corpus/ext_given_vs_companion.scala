// An extension in lexical scope wins over the companion's; a given in scope that provides the
// extension also beats the companion, since the companion belongs to the implicit scope.
class Foo
trait Ops[A]:
  extension (a: A) def show: String
object Foo:
  extension (f: Foo) def show = "companion show"
  extension (f: Foo) def tag = "companion tag"
  given Ops[Foo] with
    extension (a: Foo) def show = "given in companion"
object Lib:
  given Ops[Foo] with
    extension (a: Foo) def show = "given in lexical scope"
object Lexical:
  extension (f: Foo) def show = "lexical show"

def viaGiven(): String =
  import Lib.given
  Foo().show

def viaLexical(): String =
  import Lexical.*
  Foo().show

def viaBoth(): String =
  import Lib.given
  import Lexical.*
  Foo().show

@main def run(): Unit =
  println(Foo().tag)
  println(viaGiven())
  println(viaLexical())
  println(viaBoth())
