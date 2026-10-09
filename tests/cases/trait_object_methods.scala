// A trait's `toString`, `hashCode` and `equals` reach the classes mixing it in, where on the JVM `Object`'s
// would win over the interface's: the first of the linearisation to define one decides, a later trait over the
// one it extends, a class's own method over a trait the class extends, and a subclass of a class that defines one
// keeps that class's.
trait Named:
  override def toString: String = "named"
  override def hashCode: Int = 123
  override def equals(x: Any): Boolean = x.isInstanceOf[Named]
trait Last extends Named:
  override def toString: String = "last"
class B extends Named:
  override def toString: String = "b"
class C extends B
class D extends Last
class E extends B with Last
class F extends Named
class G extends F

@main def run(): Unit =
  println(List(new C().toString, new C().hashCode, new D().toString, new D().hashCode, new E().toString))
  println(List(new F().toString, new F().equals(new D), new G().toString, new G().hashCode, new G().equals("x")))
  val n: Named = new E
  println(List(n.toString, n.hashCode, (n: Any).equals(new C)))
