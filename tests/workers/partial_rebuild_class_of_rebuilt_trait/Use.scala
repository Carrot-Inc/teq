package inc
import Counter.given
def read: String = Counter.IntShow.show(3) + Counter.IntShow.twice(1) + summon[Show[String]].show(Stock.plain)
@main def run(): Unit = println(read)
