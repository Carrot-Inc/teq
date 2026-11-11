// An inline method of a block's class whose only reference to the enclosing instance is a type argument written in
// its body: the written type keeps its syntax, `Outer.this` a node of it, which referencesOuter finds, so the class
// has its outer accessor.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def run: Int =
    class Local:
      inline def read: Int =
        summonInline[ValueOf[Outer.this.type]].value.n
    (new Local).read
  def pair: Int =
    class Local:
      inline def read: Int =
        val o: Outer.this.type = summonInline[ValueOf[Outer.this.type]].value
        o.n + 1
    (new Local).read

@main def test(): Unit =
  println(new Outer(42).run)
  println(new Outer(42).pair)
