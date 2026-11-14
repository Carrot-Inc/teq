// Two methods of one name whose parameters are two classes of one simple name, each declared by
// its own trait and both implemented by one class or one object: a call reaches the method of
// its argument's class, on the class, through a subclass's `super` and through an export; where
// an abstract class implements one, declares the other and its subclass implements it; where a
// subclass overrides the second alone; and through the second trait's type where a subclass
// overrides the first (scalac prints `k12`, `k12`, `k1122`, `k34`, `k12`, `k56`, `k17`, `k2`).
object P1 { class Slot }
object P2 { class Slot }
trait Reads:
  def at(slot: P1.Slot): Int
trait Writes:
  def at(slot: P2.Slot): Int
class Both extends Reads with Writes:
  def at(slot: P1.Slot): Int = 1
  def at(slot: P2.Slot): Int = 2
class Swapped extends Reads with Writes:
  def at(slot: P2.Slot): Int = 2
  def at(slot: P1.Slot): Int = 1
class Sub extends Both:
  override def at(slot: P1.Slot): Int = super.at(slot) + 10
  override def at(slot: P2.Slot): Int = super.at(slot) + 20
object Single extends Reads with Writes:
  def at(slot: P1.Slot): Int = 3
  def at(slot: P2.Slot): Int = 4
object Fwd:
  val inner = new Both
  export inner.at
abstract class Base extends Reads with Writes:
  def at(slot: P1.Slot): Int = 5
  def at(slot: P2.Slot): Int
class Rest extends Base:
  def at(slot: P2.Slot): Int = 6
class Second extends Both:
  override def at(slot: P2.Slot): Int = 7
class First extends Both:
  override def at(slot: P1.Slot): Int = 8
@main def run(): Unit =
  val one = new P1.Slot
  val two = new P2.Slot
  val b = new Both
  println("k" + b.at(one) + b.at(two))
  val s = new Swapped
  println("k" + s.at(one) + s.at(two))
  val sub = new Sub
  println("k" + sub.at(one) + sub.at(two))
  println("k" + Single.at(one) + Single.at(two))
  println("k" + Fwd.at(one) + Fwd.at(two))
  val rest = new Rest
  println("k" + rest.at(one) + rest.at(two))
  val second = new Second
  println("k" + second.at(one) + second.at(two))
  val through: Writes = new First
  println("k" + through.at(two))
