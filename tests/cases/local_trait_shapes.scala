// More shapes of a local trait whose members read the values of their block: the enclosing
// instance, a class of the block defined before the trait, a local object and a class extending a
// class that mixes the trait in, a function value of the trait's type, an overloaded and a generic
// member, a default argument, a by-name parameter, a lazy val and a def with parameters of the
// block, a lambda and a local def inside a member, a trait defined inside a lambda.
class Outer(val k: Int):
  def f(n: Int): Int =
    trait T:
      def get = k + n
    (new T {}).get

trait Fn:
  def apply(x: Int): Int

def before(n: Int): Int =
  class C extends T
  trait T:
    def get = n
  (new C).get

def objects(n: Int): Int =
  trait T:
    def get = n
  object O extends T
  class C extends T
  class D extends C:
    def twice = get * 2
  O.get + (new D).twice

def sam(n: Int): Int =
  trait T extends Fn:
    def offset = n
  val t: T = x => x + 1
  t(1) + t.offset

def overloads(n: Int): String =
  trait T:
    def show(x: Int): String = s"$n:$x"
    def show(x: String): String = s"$n:$x"
    def pair[A](a: A): (Int, A) = (n, a)
    def dflt(x: Int = n): Int = x + n
  val t = new T {}
  t.show(1) + t.show("s") + t.pair("p") + t.dflt() + t.dflt(1)

def byName(n: => Int): Int =
  trait T:
    def get = n + n
  (new T {}).get

def lazies(n: Int): Int =
  lazy val l = { println("l"); n * 3 }
  def add(x: Int) = x + n
  trait T:
    def get = l + add(1)
  val t = new T {}
  t.get + t.get

def closures(n: Int): Int =
  trait T:
    def adder: Int => Int = x => x + n
    def local: Int =
      def twice(y: Int) = y * n
      twice(2)
  val t = new T {}
  t.adder(1) + t.local

def inLambda(n: Int): Int =
  val f = (m: Int) =>
    trait T:
      def get = n + m
    (new T {}).get
  f(3)

@main def run(): Unit =
  println(Outer(5).f(1))
  println(before(7))
  println(objects(3))
  println(sam(10))
  println(overloads(4))
  var calls = 0
  println(byName({ calls += 1; calls }))
  println(lazies(2))
  println(closures(5))
  println(inLambda(4))
