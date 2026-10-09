// An extractor's scrutinee, and the pair that a generator's definition is packed with for a
// guard, are held in locals named `u$0`.
def `u$0`(): Int = 7

object Half:
  def unapply(i: Int): Option[Int] = if i % 2 == 0 then Some(i / 2) else None

def half(x: Int): Int = x match
  case Half(h) => h + `u$0`()
  case _ => 0

@main def main(): Unit =
  println(half(4))
  println(for x <- List(1); y = x + 1 if y > 0 yield x + y + `u$0`())
