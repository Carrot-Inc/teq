// An export's forwarder binds the member's parameter references to its own parameters: a result and
// a later parameter typed through an earlier parameter (`def id(e: Entry): e.T`, `put(e)(v: e.T)`)
// are the argument's when the forwarder is called, as dotty's forwarder takes the member's method type.
trait Entry:
  type T
  val value: T
class Text extends Entry:
  type T = String
  val value = "ok"
class Num extends Entry:
  type T = Int
  val value = 41
class Lib:
  def id(e: Entry): e.T = e.value
  def put(e: Entry)(v: e.T): List[e.T] = List(e.value, v)
val lib = new Lib
export lib.{id, put}

object Facade:
  export lib.id as get

@main def run(): Unit =
  val e = new Text
  val s: String = id(e)
  println(s)
  val n = new Num
  val i: Int = id(n) + 1
  println(i)
  val l: List[String] = put(e)("more")
  println(l)
  val g: Int = Facade.get(n)
  println(g)
