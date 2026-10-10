object O { val x = 1 }
@main def run(): Unit = { import O.x; println(2) }
