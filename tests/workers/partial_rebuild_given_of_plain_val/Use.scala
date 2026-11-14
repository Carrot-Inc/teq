package rebuilt

def read: String = Stock.before + Stock.plain

@main def run(): Unit = println(read)
