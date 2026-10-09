trait Store:
  self =>
  type Key
  type Entry = (self.Key, String)
  def key(n: Int): self.Key
  def entry(n: Int): self.Entry = (key(n), "v" + n)

object Ints extends Store:
  type Key = Int
  def key(n: Int) = n * 10

@main def Main(): Unit =
  val e: (Int, String) = Ints.entry(4)
  println(e)
