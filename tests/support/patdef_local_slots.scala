object LocalSlots {
  def readInt(): Int = 7
  def readString(): String = "s"
  def readLong(): Long = 8L
  def readDouble(): Double = 0.5

  def c_slot_int(): Int = {
    val x = { val y = readInt(); S.out(y); y + 1 }
    S.out(x)
    x
  }

  def c_slot_reference(): String = {
    val x = { val y = readString(); S.out(y); y + "!" }
    S.out(x)
    x
  }

  def c_slot_long(): Long = {
    val x = { val y = readLong(); S.out(y); y + 1L }
    S.out(x)
    x
  }

  def c_slot_double(): Double = {
    val x = { val y = readDouble(); S.out(y); y + 1.0 }
    S.out(x)
    x
  }

  def c_slot_unit(): Any = {
    val x: Any = { val y = readInt(); S.out(y); S.out(y) }
    S.out(x)
    x
  }

  def c_slot_siblings(): Int = {
    val a = { val b = readInt(); S.out(b); b + 1 }
    val c = { val d = readInt(); S.out(d); d + 2 }
    a + c
  }
}
