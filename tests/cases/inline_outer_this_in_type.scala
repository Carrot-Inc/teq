// An inline method of a block's class whose only reference to the enclosing instance is in a type
// (`Outer.this.type`): its expansion reaches the instance through the class's outer accessor, as
// dotty's `referencesOuter` counts an outer `this` in an inline method's type trees.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def run: Int =
    class Local:
      inline def outer: Outer.this.type =
        summonInline[ValueOf[Outer.this.type]].value
    (new Local).outer.n

@main def test(): Unit = println(new Outer(42).run)
