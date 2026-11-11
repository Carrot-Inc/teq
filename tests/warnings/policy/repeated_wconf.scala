//> using options -Wconf:any:e -Wconf:any:s -Wconf:any:e
object O { def f(): Unit = { 1; () } }
@main def run(): Unit = { O.f(); println(2) }
