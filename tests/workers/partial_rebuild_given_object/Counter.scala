package rebuilt

trait Show[T]:
  def show(t: T): String

object Counter:
  given intShow: Show[Int] with
    def show(t: Int): String = "int" + t
  implicit object aliasShow extends Show[Stock.alias.type]:
    def show(t: Stock.alias.type): String = "alias" + t.size
