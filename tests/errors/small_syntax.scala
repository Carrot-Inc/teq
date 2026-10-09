// expect: secret is private to Inner
// expect: level is private to Badge
// expect: a singleton type needs a val, a parameter or an object
// expect: in a member of an object $0 is the object itself; the parameters start at $1

object Outer:
  object Inner:
    private[Inner] def secret: Int = 1
    private[Outer] def shared: Int = 2
  def fine: Int = Inner.shared
  def leak: Int = Inner.secret

class Badge(private[Badge] val level: Int)

def peek(b: Badge): Int = b.level

def compute: Int = 1
def wrong(x: compute.type): Int = 1

object Native:
  @js("[$0]") def wrap(x: Int): Any
  @js("[$1]") def fine(x: Int): Any
