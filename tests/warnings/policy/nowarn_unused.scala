import scala.annotation.nowarn
object O { val x = 1 }

@nowarn("msg=never")
def a = 1

@nowarn("id=E198")
def b = { import O.x; 2 }

@nowarn
def c = { import O.x; 3 }

@nowarn("cat=deprecation")
def d = 4

@main def run(): Unit = println(a + b + c + d)
