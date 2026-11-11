package exportedinstances
trait Render[A]:
  def render(value: A): String
object Definitions:
  given integer: Render[Int] with
    def render(value: Int): String = "value=" + value
object Published:
  export Definitions.{integer as renderer}
object Public:
  export Published.given
