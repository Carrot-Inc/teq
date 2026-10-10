object O { val x = 1 }
@scala.annotation.nowarn("msg=never-matches")
@main def run(): Unit = { import O.x; println(2) }
