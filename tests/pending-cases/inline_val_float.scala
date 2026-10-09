object Consts:
  inline val f = 0.0f

def takes(x: Float): Float = x

@main def run(): Unit =
  println(takes(Consts.f))
