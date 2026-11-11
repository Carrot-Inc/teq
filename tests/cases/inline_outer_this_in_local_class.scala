// An inline method of a class local to a method of a block's class, whose only reference to the enclosing instance
// is in a type: referencesOuter folds over the whole template, the local classes of its methods among it, so the
// block's class has its outer accessor, which the expansion's path from the local class reads.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def run: Int =
    class Local:
      def go: Int =
        class Inner:
          inline def outer: Outer.this.type =
            summonInline[ValueOf[Outer.this.type]].value
        (new Inner).outer.n
    (new Local).go
  def deeper: Int =
    class Local:
      class Member:
        def go: Int =
          class Inner:
            inline def outer: Outer.this.type =
              summonInline[ValueOf[Outer.this.type]].value
          (new Inner).outer.n + 1
      def go: Int = (new Member).go
    (new Local).go

@main def test(): Unit =
  println(new Outer(42).run)
  println(new Outer(42).deeper)
