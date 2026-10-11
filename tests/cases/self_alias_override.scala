// An alias a trait declares over a path (`type R = rdf.type` of its self alias, `this.type`) is
// seen from the class that names it, through its own `this`, as dotty's `TypeRef(C.this, R)`
// (TypeOps.asSeenFrom): `Model`'s `ROps[R]` is an `ROps[Model.type]`, which overrides the trait's
// `ROps[R]` as seen from `Model` (RefChecks.checkOverride), and a value of `R` is `Model` itself.
trait ROps[R <: RDF]:
  def name: String

trait RDF:
  rdf =>
  type R = rdf.type
  type S = this.type
  def rops: ROps[R]
  def me: S

object Model extends RDF:
  override def rops: ROps[R] = new ROps[R]:
    def name = "model"
  def me: S = this
  val self: R = this
  val same: Model.type = self

class Graph extends RDF:
  def rops: ROps[R] = new ROps[R]:
    def name = "graph"
  def me: S = this

trait GOps[R <: Store]:
  def size: Int

trait Store:
  st =>
  type R = st.type
  given ops: GOps[R]

object Mem extends Store:
  given ops: GOps[R] with
    def size = 3

@main def run(): Unit =
  println(Model.rops.name)
  println(Model.me eq Model)
  println(Model.same eq Model)
  val g = new Graph
  val r: ROps[g.type] = g.rops
  println(r.name)
  println(summon[GOps[Mem.type]].size)
