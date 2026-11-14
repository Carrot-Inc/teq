package rebuilt

def read: String = Counter.plain + Counter.other

@main def run(): Unit = println(read)
