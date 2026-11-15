package hearths

object Hearth:
  def total(n: Int): String =
    val seen = n + 1
    trait Ember:
      lazy val coal: Int = 7
      lazy val flare: Int = coal + seen
      val ash: Int = 3
    class Grate(val base: Int) extends Ember:
      lazy val both: Int = 5
      lazy val more: Int = coal + base
      object damper:
        val notch: Int = 3
        lazy val slot: Int = notch + seen
      val knob: Int = 9
      def burn(k: Int): Int = k + both + more + knob + flare + damper.notch + damper.slot
    new Grate(n).burn(1).toString
