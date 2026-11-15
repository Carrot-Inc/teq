package cpa

// Every way an object nested in a class or trait is the prefix of a compound assignment's left side: through
// a lazy value, a given, a case object, a nested class's companion, an enum's companion, a call, an array's
// element, its own class; and such an object as the receiver and as an argument of the index form; written to
// products and read again, by teq and by scalac.

class Shelf:
  object Bin:
    var n: Int = 0
    private var held: Int = 0
    def level: Int = held
    def level_=(v: Int): Unit = held = v
  object Slots:
    private var first: Int = 0
    private var second: Int = 0
    def apply(i: Int): Int = if i == 0 then first else second
    def update(i: Int, v: Int): Unit = if i == 0 then first = v else second = v
  def own(k: Int): Int = { Bin.n += k; Bin.n }

class Ledger:
  private var total: Int = 0
  def apply(key: AnyRef): Int = total
  def update(key: AnyRef, v: Int): Unit = total = v

class Desk:
  case object Tag:
    var n: Int = 0

class Outer:
  class In
  object In:
    var n: Int = 0
  enum Mode:
    case On, Off
  object Mode:
    var count: Int = 0

object Hold:
  lazy val late: Shelf = new Shelf
  given holder: Shelf = new Shelf
  val shelves: Array[Shelf] = Array(new Shelf)
  def make(): Shelf = shelves(0)
  val ledger: Ledger = new Ledger
