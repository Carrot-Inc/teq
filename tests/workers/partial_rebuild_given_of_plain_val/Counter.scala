package rebuilt

trait Show[T]:
  def show(t: T): String

object Counter:
  given intShow: Show[Int] with
    def show(t: Int): String = "int" + t
  given plainShow: Show[Stock.plain.type] with
    def show(t: Stock.plain.type): String = "plain" + t.length
