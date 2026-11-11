// The implicit scope is one list (`filterMatching`, Implicits.scala:264-269: the companion references, then the
// companions' givens), sorted by `compareEligibles` and ranked: a later candidate is tried only where
// `compareAlternatives` does not prefer the retained one, the extension methods compared only where it ties (1406-1447,
// 1517-1519). The companion object, a value, is preferred to `given (using X)`, so scalac 3.8.4 runs the companion's
// extension. teq compares the companion's extension with the given's by their receivers alone and reports an
// ambiguity; master (efddb38de3) and the head (61c1af7941) alike (apply.rs `extensions_of_scopes`, push 6's first item;
// push 5's interim review, finding 2).
class X
object X:
  given X = X()
trait Ops[A]:
  extension (a: A) def show: String
class Foo
object Foo:
  extension (f: Foo) def show = "companion"
  given (using X): Ops[Foo] with
    extension (a: Foo) def show = "given"
object Main:
  def main(args: Array[String]): Unit = println(Foo().show)
