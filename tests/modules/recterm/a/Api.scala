package rta

// A refinement's val named by a path through the refinement's own self (`b: a.type`): written
// `TERMREF a` over the refinement's `this` in a tree and over `RECthis` in an inferred type, which
// the reader keeps as the path (`RecThis.a`).
trait P:
  val a: String
  val b: Any
trait Box:
  type T
  def get: T
trait P2:
  val a: Box

object Api:
  type R = P { val b: a.type }
  type S = AnyRef { val a: String; val b: a.type }
  class K extends P:
    val a: String = "term"
    val b: a.type = a
  def make: R = K()
  def inferred =
    val r: P { val b: a.type } = K()
    r
  def read(r: R): r.a.type = r.b
  // A type member of the refinement's val through the self (`TYPEREF T` over `TERMREF a` over the
  // self: `RecThis.a.T`), and a nested refinement's members naming its own and the outer one's.
  type Q = P2 { type T = a.T }
  type N = AnyRef { type A; type B <: AnyRef { type C; type D = C; type E = A } }
  def same(q: Q)(x: q.T): q.a.T = x
  def nested(n: N)(b: n.B)(c: b.C): b.D = c
