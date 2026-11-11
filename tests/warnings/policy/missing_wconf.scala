//> using options -Wconf -deprecation
object O { def f(): Unit = { 1; () } }
@main def run(): Unit = { O.f(); println(2) }
