package rebuilt

def read: Int = Counter.take(Depot.Crate)

@main def run(): Unit = println(read)
