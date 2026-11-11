// A non-native JS class's private members are its own, whatever class the instance is (Scala.js's JSCodeGen calls a
// member PrepJSInterop does not expose statically, on its owner): a private lazy val runs its initializer on its
// first read, a subclass's private member of the same name or a member exposed by a name like the private one's does
// not take its place. The expectation is scalac 3.8.4's under Scala.js 1.22.0
// (`scala-cli run --js --js-version 1.22.0 --scala 3.8.4`).
import scala.scalajs.js
import scala.scalajs.js.annotation.JSName

class Lazy extends js.Object:
  private lazy val x: Int =
    println("init")
    7
  def read: Int = x

class A(n: Int) extends js.Object:
  private def secret: Int = n + 7
  def uses: Int = secret * 2

class B extends A(1):
  @JSName("$p_secret") def other: Int = 100

class C extends A(2):
  private def secret: Int = 1000
  def mine: Int = secret

class D extends A(3):
  @JSName("$p_A$secret") def named: Int = 100

class E(n: Int) extends js.Object:
  private def add(k: Int): Int = n + k
  def usesAdd: Int = add(3)

class F extends E(1):
  @JSName("$p_E$add") def other(k: Int): Int = 1000

@main def run(): Unit =
  println(new Lazy().read == 7)
  println(new B().uses)
  val c = new C
  println(c.uses)
  println(c.mine)
  val d = new D
  println(d.uses)
  println(d.named)
  val f = new F
  println(f.usesAdd)
  println(f.other(0))
