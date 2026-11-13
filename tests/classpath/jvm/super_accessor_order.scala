// std: scala-library
// targets: jvm
// A trait's super accessors stand in its class file in the order scalac's `SuperAccessors` makes them,
// which is the order `PostTyper` first reaches each `super` call: the statements in the template's
// order, an application's arguments before its function (an operator's right operand before its
// left), an inline call's own arguments where the call stands, a default's getter where the
// application has it, a closure's body where the closure stands, also where the closure becomes a
// class. A class that mixes such traits in holds their forwarders trait by trait, the
// first mixed in first (`ResolveSuper`), and none of a trait its superclass brings. Each line is a
// trait's or a class's `$$super$` methods read from its class file.
import java.io.DataInputStream

trait Base:
  def a(): Int = 1
  def b(): Int = 2
  def c(): Int = 3
  def n(x: Int, y: Int): Int = x - y
  def opt(x: Int = 1): Int = x
class Num(val v: Int) { def sub(x: Int, y: Int): Int = v + x - y }
def two(x: Int)(y: Int): Int = x - y
def pair(x: Int, y: Int): Int = x - y
inline def ignore(inline x: Int): Int = 0
inline def swapped(inline x: Int, inline y: Int): Int = pair(y, x)

trait Written extends Base:
  override def c(): Int = super.c() + 10
  override def a(): Int = super.a() + 10
  override def b(): Int = super.b() + 10
trait Argument extends Base:
  override def n(x: Int, y: Int): Int = super.n(super.b(), super.a())
trait Operands extends Base:
  def sum: Int = super.a() + super.b()
  def all: Boolean = (super.a() > 0 && super.b() > 0) || super.c() > 0
trait Demanded extends Base:
  def one = two1 + super.b()
  def two1 = super.a()
trait Chained extends Base:
  def text: String = "x" + super.a() + super.b()
  def holes: String = s"${super.c()}${super.n(1, 2)}"
trait Named extends Base:
  def out: Int = new Num(super.c()).sub(y = super.b(), x = super.a())
trait Positional extends Base:
  def out: Int = new Num(super.c()).sub(super.b(), super.a())
trait Lists extends Base:
  def out: Int = two(super.a())(super.b())
  def given1(x: Int = super.n(1, 2)): Int = super.c() + x
trait Dropped extends Base:
  def out: Int = { ignore(super.b().max(super.c())); super.a() }
trait Moved extends Base:
  def out: Int = swapped(super.a(), super.b())
trait GetterAfter extends Base:
  def out: Int = { super.opt(1); super.b(); super.opt() }
trait GetterRight extends Base:
  def out: Int = super.opt() + super.b()
trait GetterLeft extends Base:
  def out: Int = super.b() + super.opt()
trait Job { def go(): Int }
abstract class Task { def go(): Int }
trait Closures extends Base:
  def later: Job = () => super.b()
  def task: Task = () => super.c()
  def now: Int = super.a()
trait Lambdas extends Base:
  def out: Int = List(1).map(x => super.b() + List(2).map(y => super.a() + y).head).head
  val kept: Int = super.c()
// A trait's accessors are written where a class of the program mixes the trait in.
class OfA extends Written
class OfB extends Argument
class Of1 extends Operands
class Of2 extends Demanded
class Of3 extends Chained
class Of4 extends Named
class Of5 extends Positional
class Of6 extends Lists
class Of7 extends Dropped
class Of8 extends Moved
class Of9 extends GetterAfter
class Of10 extends GetterRight
class Of11 extends GetterLeft
class Of12 extends Lambdas
class Of13 extends Closures
trait Shared extends Base:
  override def a(): Int = super.a() + 1
trait LeftOf extends Shared:
  override def b(): Int = super.b() + 2
trait RightOf extends Shared:
  override def c(): Int = super.c() + 3
class Mixed extends Written with Argument
class MixedBack extends Argument with Written
class Three extends Demanded with Moved with Operands
class Upper extends Of2 with Moved
class Twice extends LeftOf with RightOf
class TwiceBack extends RightOf with LeftOf

def accessors(c: Class[?]): String =
  val in = new DataInputStream(c.getResourceAsStream("/" + c.getName.replace('.', '/') + ".class"))
  in.skipBytes(8)
  val n = in.readUnsignedShort()
  val names = new Array[String](n)
  var i = 1
  while i < n do
    in.readUnsignedByte() match
      case 1 => names(i) = in.readUTF()
      case 5 | 6 => in.skipBytes(8); i += 1
      case 7 | 8 | 16 | 19 | 20 => in.skipBytes(2)
      case 15 => in.skipBytes(3)
      case _ => in.skipBytes(4)
    i += 1
  in.skipBytes(6)
  in.skipBytes(2 * in.readUnsignedShort())
  def attributes(): Unit =
    var k = in.readUnsignedShort()
    while k > 0 do
      in.skipBytes(2)
      in.skipBytes(in.readInt())
      k -= 1
  def members(): List[String] =
    List.fill(in.readUnsignedShort()) {
      in.skipBytes(2)
      val name = names(in.readUnsignedShort())
      in.skipBytes(2)
      attributes()
      name
    }
  members()
  val found = members().filter(_.contains("super$")).map(m => m.substring(m.indexOf("super$") + 6))
  c.getSimpleName + ":" + found.mkString(",")

@main def run(): Unit =
  val all = List(classOf[Written], classOf[Argument], classOf[Operands], classOf[Demanded], classOf[Chained], classOf[Named],
    classOf[Positional], classOf[Lists], classOf[Dropped], classOf[Moved], classOf[GetterAfter], classOf[GetterRight],
    classOf[GetterLeft], classOf[Lambdas], classOf[Closures], classOf[Mixed], classOf[MixedBack], classOf[Three], classOf[Upper], classOf[Twice],
    classOf[TwiceBack])
  all.foreach(c => println(accessors(c)))
