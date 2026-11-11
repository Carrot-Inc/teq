//> using options -feature:help -source help
object O { def f(): Unit = { 1; () } }
@main def run(): Unit = { O.f(); println(2) }
