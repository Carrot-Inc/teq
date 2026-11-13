// A chain of inline calls, each the receiver of the next, over a super call: every term is a call that an
// expansion replaced, and the build's time does not double with each.
trait Summed extends Ground:
  def sum(v: Pt): Pt =
    super.origin() + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v + v
  def twice: Int = super.a() + super.b()
