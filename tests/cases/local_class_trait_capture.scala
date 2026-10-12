// A trait a class of a block owns reads the block's values through the enclosing instances, from
// its members and from the classes nested in it at any depth: dotty's `markFree` makes a value
// free in the largest real class between the read and the block, a trait taking none, and
// `LambdaLift.proxyRef` reads the proxy that class holds (`memberRef` through the outer path).
def deep(n: Int): Int =
  class O:
    trait T:
      class A:
        class B:
          def get = n
        def get = (new B).get
      def get = (new A).get
    def get = (new T {}).get
  (new O).get

def counter(start: Int): String =
  var count = start
  class O:
    trait T:
      def bump(by: Int): Unit = count += by
      class A:
        def twice(): Unit = { bump(1); count = count * 2 }
      def run(): Int = { (new A).twice(); count }
    def go = (new T {}).run()
  val r = (new O).go
  count += 100
  s"$r $count"

def calls(n: Int): Int =
  def helper(x: Int) = x + n
  lazy val once = { println("once"); n * 10 }
  class O:
    trait T:
      def viaDef = helper(1)
      def viaLazy = once + once
      class A:
        def both = helper(2) + once
    def get = { val t = new T {}; t.viaDef + t.viaLazy + (new t.A).both }
  (new O).get

def byName(n: => Int): Int =
  class O:
    trait T:
      def get = n + n
    def get = (new T {}).get
  (new O).get

class Host(val base: Int):
  def run(n: Int): Int =
    class O:
      trait T:
        def get = base + n + Host.this.base
      def get = (new T {}).get
    (new O).get

def twoTraits(n: Int): Int =
  class O:
    trait T1:
      trait T2:
        class A:
          def get = n + 1
        def get = (new A).get
      def get = (new T2 {}).get + n
    def get = (new T1 {}).get
  (new O).get

def inLocalTrait(n: Int): Int =
  trait LT:
    class O:
      trait T:
        def get = n * 3
      def get = (new T {}).get
    def get = (new O).get
  (new LT {}).get

def closures(n: Int): Int =
  class O:
    trait T:
      def get =
        val f = (x: Int) => x + n
        class Q:
          def q = n * 2
        f(1) + (new Q).q
    def get = (new T {}).get
  (new O).get

abstract class Base:
  def get: Int

def anonHolder(n: Int): Int =
  val b = new Base:
    trait T:
      def t = n + 7
    def get = (new T {}).t
  b.get

def givens(n: Int): String =
  given String = s"given $n"
  class O:
    trait T:
      def get = summon[String]
    def get = (new T {}).get
  (new O).get

def withDefault(n: Int): Int =
  def helper(x: Int, y: Int = n * 2) = x + y
  class O:
    trait T:
      def get = helper(1) + helper(1, 1)
    def get = (new T {}).get
  (new O).get

def withByName(n: Int): Int =
  var calls = 0
  def twice(x: => Int) = x + x
  class O:
    trait T:
      def get = twice({ calls += 1; n })
    def get = (new T {}).get
  (new O).get + calls

def objects(n: Int): String =
  object Cfg:
    val scale = n * 3
  enum Color:
    case Red, Blue
  class O:
    trait T:
      def get = s"${Cfg.scale} ${Color.Blue} ${Color.values.length}"
    def get = (new T {}).get
  (new O).get

@main def run(): Unit =
  println(deep(17))
  println(counter(3))
  println(calls(4))
  println(byName({ println("eval"); 6 }))
  println(new Host(10).run(1))
  println(twoTraits(5))
  println(inLocalTrait(4))
  println(closures(3))
  println(anonHolder(1))
  println(givens(9))
  println(withDefault(5))
  println(withByName(3))
  println(objects(2))
