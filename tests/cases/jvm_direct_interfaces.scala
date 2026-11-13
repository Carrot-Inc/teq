//> using platform jvm
// split-watch: jvm
// A class file lists the traits the class inherits directly, less those another of them brings, and
// none the superclass brings (scalac's `BCodeSkelBuilder.initJClass`); a Java interface whose default
// method a super call names is listed with them. `Picked`, whose `super[Tame].m` names a trait its
// superclass brings, is run and not listed (scalac's class file lists nothing for it). A Java interface
// the superclass brings is listed where a super call runs its default method: by `super.m`, by a trait's
// own `super.m` mixed in. Two such interfaces stand in the order scalac writes the class's bodies (the
// second line): a trait's super accessor first, then the constructor's code (a field's initialiser, a
// statement of the body), then the members in source order (a lazy val and a default argument in
// place), then the lifted bodies (a lambda, a by-name argument, a local def), an inner one first;
// a call in another's argument stands before it.
// The super accessors of several traits stand trait by trait, the first mixed in first, and in a
// trait's own order inside it (the third line).
trait Tame:
  def m: Int = 1
trait Louder extends Tame:
  override def m: Int = super.m + 1
trait Side:
  def v: Int = 5
trait Stepping extends java.util.Iterator[String]

class Plain extends Tame
class Below extends Plain
object Single extends Plain
class Again extends Plain with Tame
class Picked extends Plain with Tame:
  override def m: Int = super[Tame].m + 10
class Mixed extends Plain with Louder
class Wide extends Below with Side
class Task extends Runnable:
  def run(): Unit = ()
class Chore extends Task
case class Boxed(n: Int) extends Plain
class Unboxed extends Boxed(1)
abstract class Listing extends java.util.AbstractList[Int]
class Short extends Listing:
  def get(i: Int): Int = i
  def size(): Int = 1
class Steps extends Stepping:
  def hasNext(): Boolean = false
  def next(): String = "x"
  override def remove(): Unit =
    try super.remove() catch case _: UnsupportedOperationException => ()
object Still extends Stepping:
  def hasNext(): Boolean = false
  def next(): String = "y"
  override def remove(): Unit =
    try super.remove() catch case _: UnsupportedOperationException => ()

class Walk extends java.util.Iterator[String]:
  def hasNext(): Boolean = false
  def next(): String = "w"
class Under extends Walk:
  override def remove(): Unit =
    try super.remove() catch case _: UnsupportedOperationException => ()
object Lone extends Walk:
  override def remove(): Unit =
    try super.remove() catch case _: UnsupportedOperationException => ()
trait Careful extends java.util.Iterator[String]:
  override def remove(): Unit =
    try super.remove() catch case _: UnsupportedOperationException => ()
class Both extends Walk with Careful
abstract class Start extends java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def test(x: Int): Boolean = x > 1
  def hasNext(): Boolean = false
  def next(): Int = 0
class Calls extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def first: Boolean = List(5).map(x => super[Predicate].negate().test(x)).head
  def second: String =
    List(1).map(_ => try { super[Iterator].remove(); "removed" } catch case _: UnsupportedOperationException => "unsupported").head
def later(body: => Any): Unit = ()
trait Gentle extends java.util.Iterator[Int]:
  override def remove(): Unit = super.remove()
class Paired extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def first = () => super[Predicate].negate(); def second = () => super[Iterator].remove()
class PairedBack extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def first = () => super[Iterator].remove(); def second = () => super[Predicate].negate()
class Deferred extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def both: Unit = { later(super[Predicate].negate()); super[Iterator].remove() }
class Prompt extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def both: Unit = { super[Predicate].negate(); later(super[Iterator].remove()) }
class Above extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above = () => super[Predicate].negate()
  def below: Unit = super[Iterator].remove()
class Local extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above: Unit = { def local(): Unit = super[Predicate].negate(); local() }
  def below: Unit = super[Iterator].remove()
class Inside extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def outer = () => { super[Predicate].negate(); () => super[Iterator].remove() }
class InsideFirst extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def outer = () => { val inner = () => super[Iterator].remove(); super[Predicate].negate(); inner }
class Reached extends Start with java.util.function.Predicate[Int] with Gentle:
  def direct = super[Predicate].negate()
class ReachedLate extends Start with java.util.function.Predicate[Int] with Gentle:
  def inLambda = () => super[Predicate].negate()
class Built extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above = super[Predicate].negate()
  val below: Unit = super[Iterator].remove()
class BuiltBody extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above = super[Predicate].negate()
  super[Iterator].remove()
class BuiltTwice(val n: Int) extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above = super[Predicate].negate()
  private val kept: Unit = super[Iterator].remove()
  def this() = this(1)
class BuiltLambda extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  val above = () => super[Predicate].negate()
  def below: Unit = super[Iterator].remove()
class Idle extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above = super[Predicate].negate()
  lazy val below: Unit = super[Iterator].remove()
class IdleFirst extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  lazy val above: Unit = super[Iterator].remove()
  def below = super[Predicate].negate()
class Plainly extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above: Unit = super[Iterator].remove()
  def below = super[Predicate].negate()
class Given extends Start with java.util.function.Predicate[Int] with java.util.Iterator[Int]:
  def above(p: Unit = super[Iterator].remove()): Unit = p
  def below = super[Predicate].negate()
trait Denying extends java.util.function.Predicate[Int]:
  override def negate(): java.util.function.Predicate[Int] = super.negate()
trait Chained extends java.util.function.IntConsumer:
  override def andThen(after: java.util.function.IntConsumer): java.util.function.IntConsumer = super.andThen(after)
trait Twice extends java.util.Iterator[Int] with java.util.function.Predicate[Int]:
  override def remove(): Unit = super.remove()
  override def negate(): java.util.function.Predicate[Int] = super.negate()
trait TwiceBack extends java.util.Iterator[Int] with java.util.function.Predicate[Int]:
  override def negate(): java.util.function.Predicate[Int] = super.negate()
  override def remove(): Unit = super.remove()
abstract class Broad extends Start with java.util.function.IntConsumer:
  def accept(x: Int): Unit = ()
class Within extends Broad with java.util.Iterator[Int] with java.util.function.IntConsumer:
  def m = super[IntConsumer].andThen({ super[Iterator].remove(); this })
class Mixing extends Start with Gentle with Denying
class MixingBack extends Start with Denying with Gentle
class Three extends Broad with Gentle with Denying with Chained
class ThreeBack extends Broad with Chained with Denying with Gentle
class ThreeMixed extends Broad with Denying with Chained with Gentle
class OneTrait extends Start with Twice
class OneTraitBack extends Start with TwiceBack
@main def run(): Unit =
  def show(c: Class[?]): String = c.getSimpleName + ":" + c.getInterfaces.map(_.getSimpleName).mkString(",")
  val classes = List(classOf[Plain], classOf[Below], Single.getClass, classOf[Again], classOf[Mixed], classOf[Wide],
    classOf[Task], classOf[Chore], classOf[Boxed], classOf[Unboxed], classOf[Listing], classOf[Short], classOf[Steps], Still.getClass, classOf[Louder],
    classOf[Under], Lone.getClass, classOf[Both], classOf[Calls])
  println(classes.map(show).mkString(" "))
  val ordered = List(classOf[Paired], classOf[PairedBack], classOf[Deferred], classOf[Prompt], classOf[Above], classOf[Local], classOf[Inside],
    classOf[InsideFirst], classOf[Reached], classOf[ReachedLate], classOf[Built], classOf[BuiltBody], classOf[BuiltTwice], classOf[BuiltLambda],
    classOf[Idle], classOf[IdleFirst], classOf[Plainly], classOf[Given], classOf[Within])
  println(ordered.map(show).mkString(" "))
  val mixed = List(classOf[Mixing], classOf[MixingBack], classOf[Three], classOf[ThreeBack], classOf[ThreeMixed], classOf[OneTrait], classOf[OneTraitBack])
  println(mixed.map(show).mkString(" "))
  println(List(new Below().m, new Again().m, new Picked().m, new Mixed().m, new Wide().v, new Unboxed().m, new Short().get(3)).mkString(" "))
  new Steps().remove()
  Still.remove()
  println(new Steps().next() + Still.next())
  new Under().remove()
  Lone.remove()
  new Both().remove()
  println(s"${new Calls().first} ${new Calls().second}")
