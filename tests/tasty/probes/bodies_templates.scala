// The templates' executable roots: a parent constructor call with arguments
// and with named arguments out of order (the temporaries before the call), the template's
// statements and initialisers in order, a trait with parameters and its initialisation by a
// class, a secondary constructor, an object's statements, an anonymous class over a class with
// arguments, a local class capturing a local.
package probe.templates

def effect(s: String): Int = { print(s); s.length }

class Base(val a: Int, val b: Int):
  println("base " + a)
class Sub extends Base(effect("one"), 2):
  val x: Int = a + 1
  println("sub " + x)
  var y = x * 2
  def this(z: Int) =
    this()
    y = z
class Named extends Base(b = effect("b"), a = effect("a"))
trait Greeting(val greeting: String):
  def greet: String = greeting + "!"
class Hello extends Greeting("hello")
trait Mixed:
  val m: Int = 3
  println("mixed")
class UsesMixed extends Base(1, 2) with Mixed
object Init:
  val start: Int = effect("init")
  println("object " + start)
  def make(n: Int): Base = new Base(n, n) { override def toString = "anon" }
  def local(k: Int): Int =
    class Loc(v: Int):
      def get: Int = v + k
    new Loc(1).get
