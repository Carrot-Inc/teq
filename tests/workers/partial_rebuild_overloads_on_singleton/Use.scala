package inc
def read: String =
  Counter.pick(Shelf) + Counter.pick(Stock.alias) + Counter.pick(Other) + Counter.pick(Stock.second) + Counter.pick(3)
@main def run(): Unit = println(read)
