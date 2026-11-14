package rebuilt

trait Show[T]:
  def show(t: T): String

object Counter:
  given intShow: Show[Int] with
    def show(t: Int): String = "int" + t
  given firstShow: Show[Stock.first.type] with
    def show(t: Stock.first.type): String = "first" + t.size
  given secondShow: Show[Stock.second.type] with
    def show(t: Stock.second.type): String = "second" + t.size
