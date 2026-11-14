package rebuilt

trait Show[T]:
  def show(t: T): String

object Counter:
  type Held = Stock.alias.type
  given intShow: Show[Int] with
    def show(t: Int): String = "int" + t
  given aliasShow: Show[Held] with
    def show(t: Held): String = "alias" + t.size
