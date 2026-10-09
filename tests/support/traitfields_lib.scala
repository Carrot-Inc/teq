package tfl

import scala.annotation.targetName

// Jar traits with fields of every kind, compiled by scalac, which a class of another build
// implements as scalac's `Mixin` makes it (tests/classpath/jvm/jar_trait_fields*): the field, its
// getter and setter, a private member's under its expanded name (`tfl$Kinds$$p`), a lazy val's
// holder, an object's, and the trait's `$init$` called in linearization order.
trait Kinds:
  val v: Int = { println("init v"); 1 }
  var w: String = "w"
  private var p: Int = 10
  private val q: String = "q"
  private[this] var r: Long = 100L
  lazy val l: Int = { println("init l"); v + 40 }
  private lazy val pl: String = { println("init pl"); q + "!" }
  final val c = 5
  def bumpP(): Int = { p += 1; p }
  def bumpR(): Long = { r += 1; r }
  def privates: String = s"$q $pl $pl"
  def show: String = s"v=$v w=$w c=$c"

// A chain: a jar trait extending a jar trait with fields, each with a private var of one name.
trait Base:
  var n: Int = { println("init n"); 1 }
  private var hidden: Int = 0
  def touch(): Int = { hidden += n; hidden }

trait Mid extends Base:
  val m: Int = { println("init m"); n + 1 }
  private var hidden: String = "mid"
  def midHidden: String = { hidden += "+"; hidden }

// A val its own `$init$` reads, which a class may override (tests/classpath/jvm/trait_setter_override).
trait Over:
  val o: Int = 10
  println("Over init sees " + o)

// A val of another name in the class files (`tnv`, `tfl$Named$_setter_$tnv_$eq`), which the TASTy's member does
// not give: the class file alone gives the class the field and its accessors.
trait Named:
  @targetName("tnv") val x: Int = { println("init x"); 6 }
  def twice: Int = x * 2

// Givens: one without parameters is a lazy val, held by the class; one with a type parameter is scalac's def, made
// on every call (tests/classpath/jvm/trait_given_defs).
trait Show[A]:
  def show(a: A): String

trait Showing:
  given plain: Show[Int] = { println("make plain"); a => "plain " + a }
  given pair[A]: Show[A] with
    def show(a: A): String = "<" + a + ">"
  given alias[A]: Show[List[A]] = { println("make alias"); as => as.mkString("[", ",", "]") }

// An object member, the implementing class's lazy accessor over the trait's `Inner$`.
trait Holder:
  object Inner:
    val z: Int = { println("init Inner"); 7 }

// Statements alone: the `$init$` runs them, and every class mixing the trait in calls it.
trait Says:
  println("Says body")
  def hello: String = "hello"

// A jar class implementing a jar trait: a class of another build extending it implements nothing again.
abstract class Above extends Kinds:
  def above: String = "above " + show

// Trait parameters (tests/classpath/jvm/jar_trait_params): a val's, a private one's under its expanded name
// (`tfl$Counted$$start`), a var's with its setter, a default's getter on the trait's companion, a using
// parameter's (`tfl$Sorted$$ord`) and an anonymous one's (`tfl$AnonSorted$$x$1`), a trait of parameters alone
// (no `$init$`).
trait Counted(val label: String, start: Int, var count: Int = 0):
  println("Counted " + label + " " + start)
  val doubled: Int = start * 2
  def bump(): Int = { count += 1; count + start }

trait Sorted[A](using ord: Ordering[A]):
  def sort(xs: List[A]): List[A] = xs.sorted(using ord)

trait AnonSorted[A](using Ordering[A]):
  def sort2(xs: List[A]): List[A] = xs.sorted

trait Bare(val b: Int)

// Traits overriding `Object`'s methods (tests/classpath/jvm/jar_trait_object_methods), which a class of another
// build takes through forwarders, as scalac's `Mixin` adds them where `Object`'s would win: the first trait of the
// linearisation to define one, unless the superclass mixes it in already; a class's own stays.
trait Shown:
  override def toString: String = "shown"
  override def hashCode: Int = 123
  override def equals(x: Any): Boolean = x.isInstanceOf[Shown]

trait ShownLast extends Shown:
  override def toString: String = "shown last"

class ShownBase extends Shown:
  override def toString: String = "shown base"

// A trait's val that a class of another build inherits for its own trait's abstract val
// (tests/classpath/js/jar_val_accessor), where the build reads that val through a method, and a
// class of this jar overriding it.
trait ValBase:
  val value: Int = 3

class ValChild extends ValBase:
  override val value: Int = 9
