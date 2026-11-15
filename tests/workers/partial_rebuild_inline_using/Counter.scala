package inc
object Counter:
  transparent inline def shown(using s: Show[Int]): String = Stock.render(1)(using s)
