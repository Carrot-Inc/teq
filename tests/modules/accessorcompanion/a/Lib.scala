package acm

// An inline method of a class that reads and assigns its companion object's private members: the
// accessors are the class's (`inline$acm$C$$hidden`), reading the companion's members.
class C:
  inline def peek: Int = C.hidden + 1
  inline def add(n: Int): Int = { C.total += n; C.total }

object C:
  private val hidden = 41
  private var total = 0
