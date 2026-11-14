package rebuilt

def read: String = Stock.before + Stock.alias.size

@main def run(): Unit = println(read)
