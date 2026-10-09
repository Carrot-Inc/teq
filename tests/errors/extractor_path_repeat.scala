// expect: 11:8: warning: unreachable case
// absent: 14:8: warning
// teq: --werror
// An extractor that cannot fail repeated on one stable path (`h.ex`) is unreachable the second
// time, as scalac 3.8.4 reports it; on another path (`g.ex`) it is not.
class Ex:
  def unapply(x: Int): Some[Int] = Some(x)
class Holder(val ex: Ex)
def twice(x: Int, h: Holder): Int = x match
  case h.ex(n) => n
  case h.ex(n) => n + 1
def others(x: Int, h: Holder, g: Holder): Int = x match
  case h.ex(n) => n
  case g.ex(n) => n + 1
