// A member of a local trait that reads its block's values takes them as leading parameters, and
// every call of it passes them (dotty's `Dependencies.markFree`, `markCalled`, `computeFreeVars`,
// then `LambdaLift.addFreeParams` and `addFreeArgs`, after `Mixin`): a call through the trait
// reaches the trait's lifted method, which a class's override of the source signature does not
// override (`99` only through the class's own type), a lazy val of such a trait is its body
// evaluated at each read, a subtrait whose override calls `super` takes the values too and
// overrides the lifted method where its values are the same, and a class nested in the trait at
// any depth takes what its body reads from the member that makes it.
def dispatch(n: Int): Unit =
  trait T:
    def get = n
    lazy val once = { println("init"); n }
  class C extends T:
    override def get = 99
  val t: T = new C
  println(t.get)
  println(t.once + t.once)
  val c = new C
  println(c.get)

def deep(n: Int): Int =
  trait T:
    class A:
      class B:
        def get = n
      def get = (new B).get
    def get = (new A).get
  (new T {}).get

def supertrait(n: Int): Int =
  trait T:
    def get: Int = n
  trait U extends T:
    override def get: Int = super.get + 1
  class C extends U
  (new C).get

def calls(n: Int): String =
  trait T:
    def a = n
    def b = a * 2
  val t = new T {}
  val f = () => t.b
  def g = t.a + 1
  class K:
    def h = t.b + 100
  s"${f()} $g ${(new K).h}"

def twoValues(n: Int, m: Int): Int =
  trait T:
    def get: Int = n
  trait U extends T:
    override def get: Int = super.get + m
  class C extends U
  val c = new C
  val t: T = c
  t.get * 1000 + c.get

def init(n: Int): Int =
  trait T:
    def get = n
    val x = get + 1
  class C extends T
  (new C).x

def mutable(): String =
  var k = 1
  trait T:
    def bump(): Int = { k += 1; k }
  val t = new T {}
  t.bump()
  t.bump()
  s"$k ${t.bump()}"

@main def run(): Unit =
  dispatch(7)
  println(deep(17))
  println(supertrait(17))
  println(calls(5))
  println(twoValues(3, 4))
  println(init(9))
  println(mutable())
