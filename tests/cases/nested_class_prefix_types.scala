// A class nested in a class is a type through its prefix (dotty's `TypeRef(prefix, C)`): a path,
// a proven singleton alias of it or a projection; the enclosing class's arguments, abstract
// members and `this` are seen from the prefix (`asSeenFrom`), through a subclass's `this` too;
// an instance made through the prefix, by `new`, an import, a creator application or a secondary
// constructor, takes it as its enclosing instance; a parent through a path or an alias of one
// gives a trait its outer, and a constructor pattern is matched through the scrutinee's prefix.
class O(val n: Int):
  class Item(val v: Int):
    def outer: O.this.type = O.this
    def sum = n + v

class G[A](val a: A):
  class Inner:
    def get: A = a
  def mk: Inner = new Inner

class Sub extends G[Int](20):
  def twice: Int = (new Inner).get * 2
  def viaMk: Int = mk.get + 1
  def keep(i: Inner): Inner = i

class Abs:
  type A
  class I(val get: A)
class IntAbs extends Abs:
  type A = Int

class Sec(val n: Int):
  class I(val x: Int):
    def this() = this(1)
    def this(s: String) = this(s.length + 1)
    def get = n + x
  def fresh = new I()

class Pick(val n: Int):
  case class Foo(a: Int, b: Int)

class T7(val n: Int):
  trait I:
    def get = n
object H:
  val o = new T7(7)
type Hidden = H.o.I
class FromAlias extends Hidden
class FromPath extends H.o.I

@main def run(): Unit =
  val a = new O(1)
  val b: a.type = a
  val i: b.Item = new a.Item(2)
  val p: O#Item = new a.Item(3)
  println(i.sum + p.v)
  println(new a.Item(4).outer eq a)
  val g = new G[Int](7)
  val n: Int = (new g.Inner).get
  println(n)
  val s = new Sub
  println(s.twice)
  println(s.viaMk)
  val k: s.Inner = s.keep(s.mk)
  println(k.get + 3)
  val o = new Abs { type A = Int }
  val oi = new o.I(7)
  val m: Int = oi.get
  val ia = new IntAbs
  val m2: Int = (new ia.I(8)).get
  println(m + m2)
  val sec = new Sec(10)
  val made: sec.I = new sec.I()
  println(made.get + new sec.I("xy").get + sec.fresh.get)
  val pk = new Pick(1)
  pk.Foo(1, 2) match
    case pk.Foo(x, y) => println(x + y)
  println((new FromAlias).get + (new FromPath).get)
  locally {
    import a.Item
    val imp: a.Item = new Item(5)
    println(imp.sum + a.Item(6).sum)
  }
