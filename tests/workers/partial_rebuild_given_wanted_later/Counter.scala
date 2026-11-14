package rebuilt

trait Show[T]:
  def show(t: T): String

object Counter:
  given intShow: Show[Int] with
    def show(t: Int): String = "int" + t
  given aliasShow: Show[Stock.alias.type] with
    def show(t: Stock.alias.type): String = "alias" + t.size
