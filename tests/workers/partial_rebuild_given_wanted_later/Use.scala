package rebuilt

import Counter.given

def read: String = summon[Show[Stock.alias.type]].show(Stock.alias) + summon[Show[Int]].show(2)

@main def run(): Unit = println(read)
