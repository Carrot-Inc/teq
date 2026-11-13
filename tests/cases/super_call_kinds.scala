// Every kind of super call on a trait's method: `super[T].m`, a trait's own `super.m` (its super
// accessor in the class that mixes it in), a method two traits define (the class's forwarder), a
// super call through a class, one inside a lambda, and the own super calls of a nested class, of an
// anonymous class and of a local class in a method.
trait Base:
  def weight: Int = 1
  def name: String = "base"
trait Twice extends Base:
  override def weight: Int = super.weight * 2
trait Plus extends Base:
  override def weight: Int = super.weight + 10
trait Named:
  def name: String = "named"
trait Job:
  def go(): Int

class Stack extends Twice with Plus
class Picks extends Base with Named:
  override def name: String = super[Base].name + "/" + super[Named].name
class Deep extends Stack:
  override def weight: Int = super.weight + 100
class InLambda extends Twice:
  override def weight: Int = 7
  def all: List[Int] = List(1, 2).map(x => super.weight + x)
class InNested extends Base:
  class Inner extends Plus:
    override def weight: Int = super.weight + 100
  def viaInner: Int = new Inner().weight
class InAnon extends Base:
  def viaAnon: Int =
    val made = new Twice { override def weight: Int = super.weight + 20 }
    made.weight
  def viaLocal: Int =
    class Local extends Plus { override def weight: Int = super.weight + 30 }
    new Local().weight

@main def run(): Unit =
  println(new Stack().weight)
  println(new Picks().name)
  println(new Deep().weight)
  println(new InLambda().all)
  println(new InNested().viaInner)
  println(new InAnon().viaAnon)
  println(new InAnon().viaLocal)
