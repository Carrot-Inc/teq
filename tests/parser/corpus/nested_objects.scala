// An object nested in a class or trait is one instance per enclosing instance, made on its
// first use and holding the enclosing instance as a nested class does.
class Base(val k: Int)

class Outer(val prefix: String, n: Int):
  object Registry:
    println(s"init $prefix")
    val first = n + 1
    def show: String = prefix + first
    class Entry(val id: Int):
      def label: String = show + ":" + id

  case object Empty
  object Two extends Base(2):
    def doubled: Int = k * n

  sealed trait Shape
  case object Dot extends Shape
  final case class Box(w: Int) extends Shape

  final case class Response(text: String)
  object Response:
    given Show[Response] = r => prefix + r.text
    val schema: List[String] = List("text")

  def use: String = Registry.show + Two.doubled
  def fromLambda: List[String] = List(1, 2).map(i => Registry.show + i)
  def fromInline: Int = twice(Registry.first)
  inline def twice(inline x: Int): Int = x + x
  def entry(id: Int): String = new Registry.Entry(id).label + Registry.Entry(id + 1).label
  def shape(s: Shape): String = s match
    case Dot => "dot"
    case Box(w) => "box" + w
  def shown: String = summon[Show[Response]].show(Response("r"))
  def imported: Int =
    import Registry.*
    first

trait Show[A]:
  def show(a: A): String

trait Api:
  def base: String
  object Inner:
    println(s"inner of $base")
    def full: String = base + "!"
  def call: String = Inner.full

class ApiA extends Api:
  def base = "a"
class ApiB extends Api:
  def base = "b"

@main def run(): Unit =
  val o = Outer("#", 1)
  println(o.use)
  println(o.use)
  println(o.fromLambda)
  println(o.fromInline)
  println(o.entry(7))
  println(o.shape(o.Dot))
  println(o.shape(new o.Box(3)))
  println(o.Empty)
  println(o.Empty == o.Empty)
  println(o.shown)
  println(o.Response.schema)
  println(o.imported)
  val r: o.Registry.type = o.Registry
  println(r.show)
  val p = Outer("*", 5)
  println(p.Registry.show)
  println(p.Two.doubled + p.Two.k)
  println(o.Registry eq p.Registry)
  val a = ApiA()
  println(a.call)
  println(a.call)
  println(ApiB().call)
  val a2 = ApiA()
  println(a2.call)
  println(a.Inner eq a2.Inner)
