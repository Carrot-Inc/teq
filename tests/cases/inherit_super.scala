trait Greeter:
  def greet: String = "hello"
  def name: String

abstract class Animal(val name: String, legs: Int = 4) extends Greeter:
  def sound: String
  def describe: String = s"$name says $sound on $legs legs"
  override def greet: String = super.greet + " from " + name

class Dog(name: String) extends Animal(name):
  def sound = "woof"
  override def describe = super.describe + "!"

class Bird(name: String) extends Animal(name, legs = 2) with Greeter:
  def sound = "tweet"
  override def greet = "bird: " + super[Greeter].greet + " / " + super.greet

class Puppy extends Dog("puppy"):
  override def sound = "yip"
  override def describe = "small: " + super.describe

object Rex extends Dog("rex"):
  override def toString = "Rex!"

@main def run() =
  val animals = List(Dog("dog"), Bird("bird"), Puppy(), Rex)
  animals.foreach(a => println(a.describe))
  animals.foreach(a => println(a.greet))
  val anon = new Animal("cat", 3) {
    def sound = "meow"
    override def describe = "anon " + super.describe
  }
  println(anon.describe)
  val k = 5
  val anon2 = new Dog("d" + k) { override def sound = "k" + k }
  println(anon2.describe)
  println(Rex)
  println(anon2.isInstanceOf[Dog])
  println((anon2: Any).isInstanceOf[Greeter])
  println((Rex: Any) match { case d: Dog => "dog " + d.name; case _ => "?" })
