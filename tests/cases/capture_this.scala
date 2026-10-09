// An inline method called on a receiver that is not stable binds it to a local named `this$0`.
def `this$0`(): Int = 7

class K(val k: Int):
  inline def plus(x: Int): Int = k * 100 + x * 10 + `this$0`()

def make(): K = K(1)

@main def main(): Unit =
  println(make().plus(2))
