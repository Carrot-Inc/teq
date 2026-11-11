// An inline method of a class local to a block names an enclosing instance: its expansion outside the class reaches
// that instance from its receiver through the class's outer accessor (Inliner.computeThisBindings' outerSelect), so
// the class has one (ExplicitOuter.transformTemplate's referencesOuter under an inline method), and so has each class
// of a block between the method and the instance. A class of a block whose inline method names nothing outside it
// needs none.
class Outer(val n: Int):
  def direct: Int =
    class Local:
      inline def read: Int = Outer.this.n
    (new Local).read
  def nested: Int =
    class Local:
      class Inner:
        inline def read: Int = Outer.this.n + 1
      def inner = new Inner
    (new Local).inner.read
  def obj: Int =
    object Holder:
      inline def read: Int = n + 2
    Holder.read
  def twoLevels: Int =
    class L1:
      def go: Int =
        class L2:
          inline def read: Int = Outer.this.n + 3
        (new L2).read
    (new L1).go
  def plain: Int =
    class Quiet:
      inline def one: Int = 1
      def n2 = n * 2
    val q = new Quiet
    q.one + q.n2

@main def test(): Unit =
  val o = new Outer(40)
  println(o.direct)
  println(o.nested)
  println(o.obj)
  println(o.twoLevels)
  println(o.plain)
