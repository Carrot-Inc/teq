// A class of a block whose method reaches a local given only through a plain inline call's expansion
// (`summonInline`, searched where the inlining phase expands it) captures the given after the phase,
// and so does a subclass whose constructor calls the class's implicitly (`class Child extends Local`)
// and a subclass of that one: the free variables of a superclass constructor call are the subclass's,
// as dotty's `LambdaLift` computes them after `Inlining` (scalac prints 43 and 44).
import scala.compiletime.summonInline

trait TC:
  def get: Int

inline def summonTC: Int = summonInline[TC].get

def f(n: Int): Int =
  given TC with
    def get = n
  class Local:
    def value: Int = summonTC
  class Child extends Local
  class GrandChild extends Child:
    def more: Int = value + 1
  Child().value + GrandChild().more - GrandChild().value

@main def run(): Unit =
  println(f(42))
  println(f(42) + 1)
