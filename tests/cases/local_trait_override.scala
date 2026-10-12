// A method of a local trait that reads the block's values, overridden in a class of the block, and
// a lazy val of such a trait, as dotty lifts them after `Mixin` (`LambdaLift.addFreeArgs`): a call
// through the trait passes the values to the trait's lifted method, which the class's override of
// the source signature does not override, so it runs the trait's body (`7`, `14`) where a call
// through the class's own type runs the override (`12`, `100`), and the lazy val's initialiser
// runs at each read (`lazy 3` twice).
trait Ext:
  def get: Int

def overridden(n: Int): Int =
  trait T:
    def get = n
  class D extends T:
    override def get = super.get + 5
  val t: T = new D
  println(t.get)
  (new D).get

def through(n: Int): Int =
  trait T:
    def get = n
    def twice = get * 2
  class D extends T:
    override def get = 100
  val t: T = new D
  println(t.twice)
  (new D).twice

def external(n: Int): Ext =
  trait T extends Ext:
    def get = n
  class D extends T:
    override def get = 100
  val t: T = new D
  println(t.get)
  t

def lazies(n: Int): Int =
  trait T:
    lazy val y = { println("lazy " + n); n * 2 }
  val t = new T {}
  t.y + t.y

@main def run(): Unit =
  println(overridden(7))
  println(through(7))
  println(external(7).get)
  println(lazies(3))
