trait Logged:
  println("Logged body")
  def log(msg: String): Unit = println("log: " + msg)

trait Tagged:
  println("Tagged body")
  def tag: String = "tagged"

class Account(val owner: String, initial: Int = 0)(using scale: Int) extends Logged:
  println(s"Account($owner) body")
  var balance: Int = initial * scale
  def deposit(n: Int): Unit =
    balance += n
    log(s"$owner +$n")
  def fee: Int = 1
  def monthly(): Unit = balance -= fee

class Savings(owner: String, rate: => Int)(using Int) extends Account(initial = 5, owner = owner) with Tagged:
  println(s"Savings body, rate $rate")
  override val fee = 0
  override def deposit(n: Int): Unit =
    super.deposit(n + rate)

class Box[+T](val content: T)
class IntBox extends Box(7)
class StrBox(s: String) extends Box(s + "!")

class Counter:
  private var n = 0
  def next(): Int = { n += 1; n }

class LoudCounter extends Counter:
  override def next(): Int =
    val v = super.next()
    println("next " + v)
    v

class Base(val name: String):
  def greet: String = "hi " + name
  override def equals(that: Any): Boolean = that match
    case b: Base => b.name == name
    case _ => false
  override def hashCode: Int = name.hashCode

class Derived(name: String, val extra: Int) extends Base(name)

abstract class Animal:
  def name: String
  def intro = "I am " + name
class Cat extends Animal:
  val name = "cat"
class Dog(val name: String) extends Animal

def side(msg: String): Int = { println("eval " + msg); 1 }
class P(a: Int, b: Int):
  println("P body")
class Q extends P(side("a"), side("b")):
  println("Q body")

@main def run(): Unit =
  given Int = 10
  val s = Savings("ann", { println("rate!"); 2 })
  s.deposit(3)
  s.monthly()
  println(s.balance)
  println(s.tag)
  println(IntBox().content + 1)
  println(StrBox("a").content.length)
  val c: Counter = LoudCounter()
  c.next(); c.next()
  println(Derived("x", 1) == Derived("x", 2))
  println(Derived("x", 1) == Base("x"))
  println(Set(Derived("x", 1), Base("x")).size)
  println(Derived("x", 1).greet)
  val animals: List[Animal] = List(Cat(), Dog("rex"))
  animals.foreach(a => println(a.intro + " " + a.name))
  Q()
