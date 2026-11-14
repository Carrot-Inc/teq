package rebuilt

def read: String = "" + Stock.first.size + Stock.second.size

@main def run(): Unit = println(read)
