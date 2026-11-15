package cla

// A compound assignment's left side lifted in bodies written to products and read again: the prefix of a field
// and of a getter and setter pair, in inline methods and in a plain one; an object with an initialiser, an
// object reached through a value, a lazy value; and a companion's private member an inline method reads through
// its class's accessors.
class Slots:
  var total: Int = 0

object Tally:
  var count: Int = 0
  private var held: Int = 0
  def level: Int = held
  def level_=(v: Int): Unit = held = v

class Shelf:
  object Bin:
    var n: Int = 0
  lazy val slots: Slots = new Slots

object Ops:
  inline def bumpField(k: Int): Int = { Tally.count += k; Tally.count }
  inline def bumpSetter(k: Int): Int = { Tally.level += k; Tally.level }
  inline def bumpMember(s: Shelf, k: Int): Int = { s.Bin.n += k; s.Bin.n }
  inline def bumpLazy(s: Shelf, k: Int): Int = { s.slots.total += k; s.slots.total }
  def plain(s: Shelf, k: Int): Int = { Tally.count += k; s.Bin.n += k; Tally.count + s.Bin.n }

class Meter:
  inline def add(k: Int): Int = { Meter.sum += k; Meter.sum }

object Meter:
  private var sum = 0
