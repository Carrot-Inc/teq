// The type members of a trait's self type: an alias that holds inside the trait and on a class
// that is the self type, a member bounded by itself (`type T <: Element[T]`) seen through the
// trait, and a member reached on an intersection receiver.
package selftypemember

class ActorRef(val name: String)
trait ActorEventBus:
  type Subscriber = ActorRef
trait ManagedActorClassification:
  this: ActorEventBus =>
  var subs: List[Subscriber] = Nil
  def subscribe(s: Subscriber): Unit = subs = s :: subs
  def names: List[String] = subs.map(_.name)
class Bus extends ManagedActorClassification with ActorEventBus

trait Element[T]
trait Config:
  type T <: Element[T]
  def make: T
trait Transform:
  self: Config =>
  def twice: List[T] = List(make, make)
class E(val n: Int) extends Element[E]
object Cfg extends Config with Transform:
  type T = E
  def make: E = new E(7)

trait Literal:
  type F[X]
  def num(i: Int): F[Int]
trait Addition:
  self: Literal =>
  def add(l: F[Int], r: F[Int]): F[Int]
object Lit extends Addition with Literal:
  type F[X] = List[X]
  def num(i: Int): List[Int] = List(i)
  def add(l: List[Int], r: List[Int]): List[Int] = l ++ r

object Main:
  def expression(adder: Addition & Literal): adder.F[Int] = adder.add(adder.num(1), adder.num(2))
  def main(args: Array[String]): Unit =
    val bus = new Bus
    bus.subscribe(new ActorRef("a"))
    bus.subscribe(new ActorRef("b"))
    println(bus.names)
    println(Cfg.twice.map(_.n))
    println(expression(Lit))
