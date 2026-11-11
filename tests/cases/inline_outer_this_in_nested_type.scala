// An inline method of a class nested in a block's class whose only reference to the enclosing instance is in a
// type: the block's class has its outer accessor, as dotty's referencesOuter folds over the whole template.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def run: Int =
    class Local:
      class Inner:
        inline def outer: Outer.this.type =
          summonInline[ValueOf[Outer.this.type]].value
      def inner = new Inner
    (new Local).inner.outer.n

@main def test(): Unit = println(new Outer(42).run)
