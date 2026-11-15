package relays

class Coil(val turns: Int):
  def pull(k: Int): Int = k * turns
  def pull(k: String): Int = k.length + turns
  def mark(k: Int, pad: Int = 2): String = ("0" * pad) + k
  val label: String = "c" + turns

class Switch(c: Coil):
  export c.{pull, mark, label}
  def all(k: Int): String = s"${pull(k)},${pull("ab")},${mark(k)},${mark(k, 1)},$label"

object Board:
  private val base = new Coil(3)
  export base.*

trait Wired:
  val inner: Coil
  export inner.pull as tug

object Relay:
  def total(n: Int): String =
    val s = new Switch(new Coil(n + 1))
    val w = new Wired { val inner = new Coil(n + 2) }
    s"${s.all(n)},${s.pull(5)},${Board.pull(n)},${Board.mark(n)},${Board.label},${w.tug(4)},${w.tug("xyz")}"
