package inc
object Counter:
  object IntShow extends Show[Int]:
    def show(t: Int): String = "int" + t
  given strShow: Show[String] with
    def show(t: String): String = "str" + t
