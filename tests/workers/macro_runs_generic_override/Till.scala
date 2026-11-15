package tills

trait Drawer[A]:
  def take(k: A): Int
  def take(k: String): Int

object Till:
  def total(n: Int): Int =
    class Tray(val start: Int) extends Drawer[Int]:
      def take(k: Int): Int = start + k
      def take(k: String): Int = start + k.length + 100
    val d: Drawer[Int] = new Tray(n)
    d.take(3) + d.take("four")
