package p.amount
opaque type Amount = Long
object Amount:
  def zero: Amount = 0L
  extension (a: Amount)
    def -(b: Amount): Amount = a - b
    def isNegative: Boolean = a < 0L
