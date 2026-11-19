package outside

trait Left:
  self: use.Impl =>
  def f: Int = g

trait Right:
  self: lib.inner.B =>
  def h: Int = m
