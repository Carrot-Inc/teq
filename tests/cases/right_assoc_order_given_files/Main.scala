given si: Show[Int] = { println("make the Int's"); new Show[Int] { def show(t: Int): String = "i" + t } }
given sb: Show[Boolean] = { println("make the Boolean's"); new Show[Boolean] { def show(t: Boolean): String = "b" + t } }
case class Zed(x: Boolean, y: Int) derives Show
@main def run(): Unit = println(summon[Show[Zed]].show(Zed(true, 1)))
