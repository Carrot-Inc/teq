package acl

// Inline methods that read and assign private members of their class, and a protected and a
// qualified-private one: a downstream expands them through the accessors the upstream's
// pickles and class files give the classes (`inline$..`).
class Counter(start: Int):
  private var count = start
  private def bump(n: Int): Int = { count += n; count }
  protected def step: Int = 1
  private[acl] def scoped: Int = 2
  inline def tick(n: Int): Int = bump(n) + count + step + scoped

object Holder:
  private def twice(x: => Int): Int = x + x
  inline def runTwice(x: => Int): Int = twice(x)
  private val secret = 41
  private var total = 0
  inline def peek: Int = secret + 1
  inline def add(n: Int): Int = { total += n; total }
