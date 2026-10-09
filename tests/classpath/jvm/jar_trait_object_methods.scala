// jars: scala-library traitfields-lib
// A class mixing in a jar trait that overrides `toString`, `hashCode` or `equals` takes the trait's, as scalac's
// `Mixin` forwards them (`MixinOps.needsMixinForwarder`: `Object`'s would win on the JVM): the first trait of the
// linearisation to define one, a later trait's over the one it extends; a class's own method stays, and a class
// whose superclass mixes the trait in already inherits the superclass's. master took `Object`'s for all of them.
import tfl.*

class C extends Shown
class D extends ShownLast
class Own extends Shown:
  override def toString: String = "own"
class Sub extends ShownBase
class Again extends ShownBase with ShownLast
class Pair extends C

@main def run(): Unit =
  val c = new C
  println(List(c.toString, c.hashCode, c.equals(new C), c.equals("x"), c == new D))
  val d = new D
  println(List(d.toString, d.hashCode, d.equals(c)))
  println(List(new Own().toString, new Own().hashCode))
  println(List(new Sub().toString, new Sub().hashCode, new Again().toString, new Pair().toString, new Pair().hashCode))
  val shown: Shown = new Again
  println(List(shown.toString, shown.hashCode, (shown: Any).equals(c)))
