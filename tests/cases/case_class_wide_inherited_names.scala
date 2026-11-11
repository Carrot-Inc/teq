// A wide written parent shares names with the case class's Product/Equals parents.
trait Wide:
  override def toString: String = "wide"
  override def hashCode: Int = 31
  override def equals(other: Any): Boolean = false
  def member00: Int = 0
  def member01: Int = 1
  def member02: Int = 2
  def member03: Int = 3
  def member04: Int = 4
  def member05: Int = 5
  def member06: Int = 6
  def member07: Int = 7
  def member08: Int = 8
  def member09: Int = 9
  def member10: Int = 10
  def member11: Int = 11
  def member12: Int = 12
  def member13: Int = 13
  def member14: Int = 14
  def member15: Int = 15
  def member16: Int = 16
  def member17: Int = 17
  def member18: Int = 18
  def member19: Int = 19
  def member20: Int = 20
  def member21: Int = 21
  def member22: Int = 22
  def member23: Int = 23
  def member24: Int = 24
  def member25: Int = 25
  def member26: Int = 26
  def member27: Int = 27
  def member28: Int = 28
  def member29: Int = 29
  def member30: Int = 30
  def member31: Int = 31
  def member32: Int = 32
  def member33: Int = 33
  def member34: Int = 34
  def member35: Int = 35
  def member36: Int = 36
  def member37: Int = 37
  def member38: Int = 38
  def member39: Int = 39
  def member40: Int = 40
  def member41: Int = 41
  def member42: Int = 42
  def member43: Int = 43
  def member44: Int = 44
  def member45: Int = 45
  def member46: Int = 46
  def member47: Int = 47
  def member48: Int = 48
  def member49: Int = 49
  def member50: Int = 50
  def member51: Int = 51
  def member52: Int = 52
  def member53: Int = 53
  def member54: Int = 54
  def member55: Int = 55
  def member56: Int = 56
  def member57: Int = 57
  def member58: Int = 58
  def member59: Int = 59
  def member60: Int = 60
  def member61: Int = 61
  def member62: Int = 62
  def member63: Int = 63
  def member64: Int = 64
  def member65: Int = 65
  def member66: Int = 66
  def member67: Int = 67
  def member68: Int = 68
  def member69: Int = 69
  extension (text: String) def textWidth: Int = text.length

case class Sample(value: Int) extends Wide

@main def run(): Unit =
  val sample = Sample(3)
  println(sample)
  println(sample.hashCode)
  println(sample == Sample(3))
  println(sample.member69)
  println(sample.productArity)
  import sample.*
  println("four".textWidth)
