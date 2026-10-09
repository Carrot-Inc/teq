// expect: type mismatch: found String, required Int
// expect: not found: missing
// expect: value nope is not a member of Int
// expect: no given instance of type Show[Boolean]
// expect: reassignment to val x
// expect: missing argument for parameter b
// expect: too many arguments
// expect: Animal is a trait; it cannot be instantiated
// expect: class Cat needs to be abstract, since def sound: String in trait Animal is not defined
// expect: recursive use of loop needs an explicit result type
// expect: cannot be compared with == or !=
// expect: secret is private to Vault
// expect: pin is private to Vault

trait Show[A]:
  def show(a: A): String

given Show[Int] with
  def show(a: Int): String = a.toString

def shown[A](a: A)(using s: Show[A]): String = s.show(a)

trait Animal:
  def sound: String

class Base
class Derived extends Base

class Cat extends Animal

trait Stateful:
  val counter: Int = 0
  lazy val fine: Int = 1
  def alsoFine: Int = 2

class Vault(pin: Int):
  private val secret: Int = pin * 2
  def check(guess: Int): Boolean = guess == secret

def add(a: Int, b: Int): Int = a + b

def loop(n: Int) = if n == 0 then 0 else loop(n - 1)

@main def run(): Unit =
  val x: Int = "hello"
  println(missing)
  println(1.nope)
  println(shown(true))
  x = 5
  println(add(1))
  println(add(1, 2, 3))
  val a = Animal()
  println(1 == "one")
  val v = Vault(1234)
  println(v.secret)
  println(v.pin)
