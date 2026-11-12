//> using platform jvm
// (JavaScript left out.)
// A `return` in a local lazy val's initialiser leaves the method declaring the val wherever the val is forced: in an
// anonymous class, a named local class, a local trait's member and initialiser, a super call to it, a local class's
// default argument, a local def's copy, a lambda inside a class's method, a class made by a lambda, a class made in a
// class (dotty's `NonLocalReturns` key of that method, a local val LambdaLift passes on as it passes the holder); a
// lambda that has a return of its own beside the read throws each to its method; an unforced val returns nothing.
// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
import java.util.concurrent.Callable
object Returns:
  // the initialiser's return leaves the method declaring the val, through an anonymous class, a named class, a
  // local trait, a lambda inside a class's method, a lambda that has its own return beside
  def anon(): Int =
    lazy val x: Int = return 1
    new Runnable { def run(): Unit = println(x) }.run()
    0
  def named(): Int =
    lazy val x: Int = return 2
    class L { def run(): Unit = println(x) }
    new L().run()
    0
  def viaTrait(): Int =
    lazy val x: Int = return 3
    trait R { def read(): Int = x }
    class C extends R
    new C().read()
    0
  def lambdaInClass(): Int =
    lazy val x: Int = return 4
    new Runnable { def run(): Unit = List(1).foreach(_ => println(x)) }.run()
    0
  def twoKeys(): Int =
    lazy val x: Int = return 5
    class L:
      def run(first: Boolean): Int =
        List(1).foreach(i => if first then return 50 else println(x))
        60
    println(new L().run(true))
    new L().run(false)
    0
  def notForced(): Int =
    lazy val x: Int = return 6
    new Runnable { def run(): Unit = () }.run()
    7
  def inner(): Int =
    lazy val x: Int = return 8
    class A:
      def make(): Runnable = new Runnable { def run(): Unit = println(x) }
    new A().make().run()
    0

object Returns2:
  def traitInit(): Int =
    lazy val x: Int = return 1
    trait T { val v: Int = x }
    class C extends T
    new C().v
    0
  def defaultArg(): Int =
    lazy val x: Int = return 2
    class L(val a: Int = x)
    new L().a
    0
  def superCall(): Int =
    lazy val x: Int = return 3
    trait T { def get(): Int = x }
    class C extends T { override def get(): Int = super.get() + 1000 }
    new C().get()
    0
  def viaDef(): Int =
    lazy val x: Int = return 4
    def d(): Int = x
    new Callable[Int] { def call(): Int = d() }.call()
    0
  def inLambdaWithClass(): Int =
    lazy val x: Int = return 5
    val f = () => new Callable[Int] { def call(): Int = x }
    f().call()
    0

object Main:
  def main(args: Array[String]): Unit =
    println(List(Returns.anon(), Returns.named(), Returns.viaTrait(), Returns.lambdaInClass(), Returns.twoKeys(), Returns.notForced(), Returns.inner()))
    println(List(Returns2.traitInit(), Returns2.defaultArg(), Returns2.superCall(), Returns2.viaDef(), Returns2.inLambdaWithClass()))
