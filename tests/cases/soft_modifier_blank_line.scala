class SoftName(val into: Boolean):
  def value: Boolean =
    into

  def next = 1
@main def readSoftName(): Unit =
  println(new SoftName(true).value)
  println(new SoftName(false).next)
