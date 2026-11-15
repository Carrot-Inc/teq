package vaults

trait Lock:
  def turn(k: Int): Int
  def turn(k: String): Int = k.length

object Vault:
  def total(n: Int): Int =
    val seen = n + 1
    var turns = 0
    class Door(val start: Int) extends Lock:
      lazy val hinge: Int = start + seen
      def turn(k: Int): Int =
        class Bolt(val v: Int):
          def twice: Int = v * 2
        turns += 1
        new Bolt(start + k + seen).twice
      object latch:
        val notch: Int = 3
      def ease: Int = start + latch.notch
      def open(k: Int): Int = start + k + seen
      def open(k: String): Int =
        val r = new Runnable { def run(): Unit = turns += 10 }
        r.run()
        start + k.length
    object Safe:
      def put(k: Int): Int = k + seen
      def put(k: String): Int = k.length
    val d = new Door(n)
    val l: Lock = d
    d.hinge + d.ease + l.turn(3) + l.turn("four") + d.open(1) + d.open("ab") + Safe.put(2) + Safe.put("abc") + turns
