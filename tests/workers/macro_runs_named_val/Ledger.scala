package scales

trait Scale:
  def weight(step: Int): Int = step

object Ledger:
  def total(n: Int): Int =
    class Pan(val start: Int) extends Scale:
      val weight: Int = 5
      def sum: Int = start + weight + weight(2)
    new Pan(n).sum
