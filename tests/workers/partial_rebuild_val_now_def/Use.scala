package rebuilt

def read: String = "" + Counter.take(Shelf) + Counter.held.size

@main def run(): Unit = println(read)
