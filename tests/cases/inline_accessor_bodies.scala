// The accessors an inline method reads private and protected members through (PrepareInlineable, AccessProxies'
// accessorDef) are the class's members in every build, each forwarding to its member on `this` or on its object: a
// constructor parameter, a parameterless method (its call), a field and a var, a trait's protected method, an object's
// private val and method, a companion's private; an inline method never called still has its accessors.
class Late(x: Int):
  def plain: Int = x + 1
  inline def peek: Int = x

class Calc:
  private def dbl = 21
  private val fld = 1
  inline def twice: Int = dbl * 2 + fld

trait Counted:
  private var n = 0
  protected def base: Int = 40
  inline def inc: Int = { n += 1; n + base }

class K extends Counted

object Cache:
  private val cached = 5
  private def twice(x: Int) = x * 2
  inline def get: Int = twice(cached)

class Holder(private val secret: Int):
  private var hits = 0
  inline def get: Int = { hits += 1; Holder.base + secret + hits }

object Holder:
  private val base = 100
  inline def peek(h: Holder): Int = h.secret * 2

@main def run(): Unit =
  println(new Late(3).plain)
  println(new Calc().twice)
  val k = new K
  k.inc
  println(k.inc)
  println(Cache.get)
  val h = new Holder(5)
  println(s"${Holder.peek(h)} ${h.get} ${h.get}")
