// An overloaded inline method of a block's class whose only reference to the enclosing instance is in a type:
// every alternative is an inline DefDef referencesOuter folds over, so the class has its outer accessor.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def run: Int =
    class Local:
      inline def outer: Outer.this.type =
        summonInline[ValueOf[Outer.this.type]].value
      inline def outer(i: Int): Int = i
    (new Local).outer.n

@main def test(): Unit = println(new Outer(42).run)
