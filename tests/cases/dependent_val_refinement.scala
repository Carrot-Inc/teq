// A method whose result refines a val with its parameter's singleton type: the call's result
// has the argument's path in the refinement, which a parameter declared over that path accepts.
trait Q:
  def name: String
trait Insp:
  val qctx: Q
object Insp:
  def make(q: Q): Insp { val qctx: q.type } = new Insp { val qctx: q.type = q }
object Holder:
  val qctx: Q = new Q { def name = "q1" }
  def run(i: Insp { val qctx: Holder.qctx.type }): String = i.qctx.name
  def go: String = run(Insp.make(qctx))

object Main:
  def main(args: Array[String]): Unit =
    val q: Q = new Q { def name = "q2" }
    val i: Insp { val qctx: q.type } = Insp.make(q)
    println(Holder.go + " " + i.qctx.name + " " + Chain.show)

object Chain:
  def next(i: Insp): Insp { val qctx: i.qctx.type } = Insp.make(i.qctx)
  def via(i: Insp { val qctx: Holder.qctx.type }): String =
    val j: Insp { val qctx: Holder.qctx.type } = next(i)
    val q: Q = j.qctx
    q.name
  def show: String = via(Insp.make(Holder.qctx))
