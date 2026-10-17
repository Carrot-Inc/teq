package icb

import ica.{Mac, Pair, Plain}

// No code of this module calls `dead`: the later module does.
object Made:
  val anon: Pair = Mac.anon
  val local: Pair = Mac.local
  val nested: Pair = Mac.nested
  val plain: Plain = Mac.plain

  // The module's own classes are its products' too, whole.
  val own: Pair = new Pair:
    extension (x: Int)
      def live: Int = x + 40
      def dead: Int = x + 50
    def bonus: Int = 60

  def ownLocal: Pair =
    class Mine extends Pair:
      extension (x: Int)
        def live: Int = x + 70
        def dead: Int = x + 80
      def bonus: Int = 90
    Mine()

  val withUnused: Pair = Mac.withUnused
  val picked: Pair = Mac.pick(true)

  def touch: List[Int] = List(anon, local, nested, own, ownLocal, withUnused, picked).map(_.live(0)) :+ plain.live
