// A parameterless structural given is a module (`Parsers.givenDef` makes it a `ModuleDef`): its name reads
// its class, so the members only its body has are selected on it (`t.extra`, through a val too), and inside
// its body its name and a search for its type are the class's `this` (`tpd.ref`), so a val that reads it
// while the instance is made gets the instance, at the top level, in an object and in a class. An object
// nested in a class read by its name in its own body is its `this` as well.
trait T { val self: T; def n: Int }
def get(using x: T): T = x
given t: T with
  def n = 1
  val self: T = summon[T]
  val named: T = t
  val viaUsing: T = get
  def extra: String = "extra"
object O:
  given u: T with
    def n = 2
    val self: T = summon[T]
class C:
  given v: T with
    def n = 3
    val self: T = summon[T]
  object M extends T:
    def n = 4
    val self: T = M
@main def run(): Unit =
  println(t.self eq t)
  println(t.named eq t)
  println(t.viaUsing eq t)
  println(t.extra)
  println(O.u.self eq O.u)
  val c = new C
  println(c.v.self eq c.v)
  println(c.M.self eq c.M)
  val x = t
  println(x.extra + x.n)
