package exd

// A top-level export of a member whose result and later parameter are typed through an earlier
// parameter: the forwarder's signature binds them to its own parameters, in the products too.
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
