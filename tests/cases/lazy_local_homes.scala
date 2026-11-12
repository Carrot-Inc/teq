// A local lazy val's accessor and initialiser, which stay methods of the class whose code declares the val, and a
// class's copy of a local def read that class's instance where the code reads `this` (dotty's LambdaLift lifts both
// into that class and calls them through its instance): from an anonymous class, a named local class, a local
// trait's member and initialiser, a
// local class's default argument, a local case class's `copy`, a super call to a local trait's member, a class made
// inside a class or inside a lambda, a local class over a local class, a local object, a def of a local class's
// method reading a def of the enclosing method, a lambda in the initialiser; an object's own reached as `MODULE$`.
import java.util.concurrent.Callable
trait Reader { def read(): Int }
class Outer(val n: Int):
  // a lazy val reading `this`, forced by an anonymous class, by a named local class, by a local trait's member
  def anon(): Int =
    lazy val x = n + 1
    val r = new Callable[Int] { def call(): Int = x }
    r.call() + x
  def named(): Int =
    lazy val x = n + 2
    class L extends Reader { def read(): Int = x }
    new L().read()
  def viaTrait(): Int =
    lazy val x = n + 3
    trait T { def get(): Int = x }
    class C extends T
    new C().get()
  // a local def reading `this`, called by an anonymous class and a local trait
  def defAnon(): Int =
    def d(): Int = n + 4
    val r = new Callable[Int] { def call(): Int = d() }
    r.call()
  def defTrait(): Int =
    def d(): Int = n + 5
    trait T { def get(): Int = d() }
    class C extends T
    new C().get()
  // a class made inside a class made inside the method, the inner one forcing the val
  def nested(): Int =
    lazy val x = n + 6
    class A:
      def make(): Reader = new Reader { def read(): Int = x }
    new A().make().read()
  // a class made inside a lambda of the method
  def inLambda(): Int =
    lazy val x = n + 7
    val f = () => new Reader { def read(): Int = x }
    f().read()
  // a lazy val of a local class's method, reading that class's `this`, forced by a class made there
  def localHome(): Int =
    class L(val m: Int):
      def get(): Int =
        lazy val y = m + n
        new Reader { def read(): Int = y }.read()
    new L(100).get()
  // a lazy val reading `this` and returning from its method, forced in a class
  def both(b: Boolean): Int =
    lazy val x: Int = if b then return n + 8 else n + 9
    new Reader { def read(): Int = x }.read()

class Outer2(val n: Int):
  // a local trait's initialiser reading the val
  def traitInit(): Int =
    lazy val x = n + 1
    trait T { val v: Int = x }
    class C extends T
    new C().v
  // a local class's default argument reading the val
  def defaultArg(): Int =
    lazy val x = n + 2
    class L(val a: Int = x)
    new L().a
  // a local case class's copy, the class holding the val's home
  def caseCopy(): Int =
    lazy val x = n + 3
    case class P(a: Int) { def total: Int = a + x }
    P(1).copy(a = 2).total
  // a super call to a local trait's member
  def superCall(): Int =
    lazy val x = n + 4
    trait T { def get(): Int = x }
    class C extends T { override def get(): Int = super.get() + 1000 }
    new C().get()
  // a local class over a local class, both reading the val
  def overLocal(): Int =
    lazy val x = n + 5
    class A { def a: Int = x }
    class B extends A { def b: Int = x * 2 }
    val b = new B
    val c = new A { override def a: Int = x * 3 }
    b.a + b.b + c.a
  // a local object reading this, forced by an anonymous class
  def localObject(): Int =
    object O { def v: Int = n + 6 }
    new Callable[Int] { def call(): Int = O.v }.call()
  // two homes: a def of the method, read by a def of a local class's method, copied into a class there
  def twoHomes(): Int =
    def outerDef(): Int = n + 7
    class L(val m: Int):
      def get(): Int =
        def innerDef(): Int = outerDef() + m
        new Callable[Int] { def call(): Int = innerDef() }.call()
    new L(1000).get()
  // a lambda in the initialiser reading this
  def lambdaInInit(): Int =
    lazy val x = List(1, 2).map(_ + n).sum
    new Callable[Int] { def call(): Int = x }.call()

object Obj:
  val k = 10
  def defInObject(): Int =
    def d(): Int = k + 1
    new Callable[Int] { def call(): Int = d() }.call()
  def lazyInObject(): Int =
    lazy val x = k + 2
    new Callable[Int] { def call(): Int = x }.call()

object Obj2:
  val k = 10
  def lambdaInInit(): Int =
    lazy val x = List(1, 2).map(_ + k).sum
    new Callable[Int] { def call(): Int = x }.call()
  def lambdaInDef(): Int =
    def d(): Int = List(1).map(_ + k).sum
    new Callable[Int] { def call(): Int = d() }.call()

object Main:
  def main(args: Array[String]): Unit =
    val o = new Outer(41)
    println(List(o.anon(), o.named(), o.viaTrait(), o.defAnon(), o.defTrait(), o.nested(), o.inLambda(), o.localHome(), o.both(true), o.both(false)))
    val o2 = new Outer2(41)
    println(List(o2.traitInit(), o2.defaultArg(), o2.caseCopy(), o2.superCall(), o2.overLocal(), o2.localObject(), o2.twoHomes(), o2.lambdaInInit()))
    println(List(Obj.defInObject(), Obj.lazyInObject(), Obj2.lambdaInInit(), Obj2.lambdaInDef()))
