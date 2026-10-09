package vab

class Over extends vaa.Holder:
  override val v: Int = 5

@main def run(): Unit =
  println(new vaa.Holder().twice)
  println(new Over().twice)
