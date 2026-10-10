// A private var of a class or object has no setter (dotty's `Desugar.isSetterNeeded`), so its
// `_=` is no member; a trait's private var has one.
// expect: 6:34: error: value n_= is not a member of P
// expect: 1 error found
object P:
  private var n = 0; def set() = this.n_=(1); def get = n
trait Q:
  private var m = 0
  def set(): Unit = this.m_=(1)
  def get = m
@main def run(): Unit = { P.set(); println(P.get) }
