//> using options -Wconf:any:s,help
object O { def f(): Unit = { 1; () } }
@main def run(): Unit = { O.f(); println(2) }
