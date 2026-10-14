// A class's base type for an ancestor its parents give at several instantiations is their meet (dotty's
// `SymDenotations.baseTypeOf`'s `foldGlb` over the parents, `TypeComparer.glbArgs`): a covariant argument's glb
// (`Sub1 & Sub2`), a contravariant one's lub (`String | Int`). The arguments here name subclasses of the class
// itself, which complete after it: the meet waits for no completion to be under way, and the subclasses, whose
// base types copied the instance the class held until then, take the meet of their parents' again.
trait Holder[+T]:
  def value: T
trait First extends Holder[Sub1]
trait Second extends Holder[Sub2]
trait Both extends First with Second
trait Sub1 extends Both:
  def name: String = "sub1"
trait Sub2 extends Both:
  def other: String = "sub2"
class Leaf extends Sub1 with Sub2:
  def value: Sub1 & Sub2 = this

def first(b: Both): Sub1 = b.value
def meet(b: Both): Sub1 & Sub2 = b.value
def throughSub(s: Sub1): Sub1 & Sub2 = s.value

trait Sink[-A]:
  def put(a: A): Unit
trait TextSink extends Sink[String]
trait IntSink extends Sink[Int]
trait BothSinks extends TextSink with IntSink
class Printer extends BothSinks:
  def put(a: String | Int): Unit = println(s"put $a")

trait A
trait B
trait AH extends Holder[A]
trait BH extends Holder[B]
trait BothAB extends AH with BH
class AB extends BothAB with A with B:
  def value: A & B = this

@main def main(): Unit =
  val leaf = new Leaf
  println(first(leaf).name)
  println(meet(leaf).other)
  println(throughSub(leaf).name)
  val sinks: BothSinks = new Printer
  sinks.put("hello")
  sinks.put(1)
  val ab: BothAB = new AB
  val both: A & B = ab.value
  println(both == ab)
