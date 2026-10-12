// A local trait whose members read the values of the enclosing block: its methods take what they
// read as leading parameters and each class mixing it in holds those values and forwards to them
// (dotty's Dependencies and LambdaLift, the forwarders of its Mixin): directly, through a named or an
// anonymous class, through a local def, from the trait's initialiser, a captured var shared by
// reference across instances escaping the call, a trait nested in a local class, a chain of
// local traits, a class and an object nested in the trait, a super call, a type parameter and a
// given of the block.
trait Counter:
  def next(): Int

trait Show:
  def show: String

def direct(n: Int): Int =
  trait T:
    def get = n
  (new T {}).get

def named(n: Int): Int =
  trait T:
    def get = n
  class C extends T
  (new C).get

def throughDef(n: Int): Int =
  def get = n
  trait T:
    def value = get
  class C extends T
  (new C).value

def initialiser(n: Int): Int =
  trait T:
    val x = { println(n); n + 1 }
  class C extends T
  val a = new C
  val b = new T {}
  a.x + b.x

def make(n: Int): Counter =
  var x = n
  trait T extends Counter:
    def next(): Int = { x += 1; x }
  class C extends T
  new C

def shared(): () => Int =
  var n = 0
  trait T:
    def next(): Int = { n += 1; n }
  val a = new T {}
  val b = new T {}
  () => a.next() + b.next()

def nested(n: Int): Int =
  class C:
    trait T:
      def get = n
    def get = (new T {}).get
  (new C).get

def chain(n: Int, m: Int): Int =
  trait A:
    def a = n
  trait B extends A:
    def b = a + m
  class C extends B
  (new C).b

def inner(n: Int): Int =
  trait T:
    class Inner:
      def get = n
    object O:
      def get = n + 100
    def make = new Inner
  val t = new T {}
  t.make.get + t.O.get

def supers(n: Int): Int =
  trait T:
    def get = n
  class D extends T:
    override def get = super.get * 10
  (new D).get

def escapes(n: Int): Show =
  trait T extends Show:
    def show = "T" + n
  new T {}

def generic[A](a: A): String =
  trait T:
    def get: A = a
  (new T {}).get.toString

def givens(n: Int): Int =
  given Int = n
  trait T:
    def get = summon[Int]
  (new T {}).get

def vars(n: Int): Int =
  trait T:
    var z = n + 1
    def bump(): Int = { z += n; z }
  val t = new T {}
  t.bump()
  t.z

def twice(n: Int): Int =
  trait T:
    def get = n
    def twice = get * 2
  class C extends T
  (new C).twice

@main def run(): Unit =
  println(direct(7))
  println(named(7))
  println(throughDef(7))
  println(initialiser(7))
  val a = make(1)
  val b = make(10)
  println(a.next())
  println(b.next())
  println(a.next())
  val s1 = shared()
  val s2 = shared()
  println(s1())
  println(s1())
  println(s2())
  println(nested(7))
  println(chain(1, 2))
  println(inner(5))
  println(supers(4))
  println(escapes(7).show + escapes(8).show)
  println(generic("g"))
  println(givens(9))
  println(vars(3))
  println(twice(4))
