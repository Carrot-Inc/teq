// teq: --release
// Under `--release` a constructor's parameter goes by the short name of its field (`a`): a local
// of the constructor named like it is named apart instead of hiding it.
class C(longName: Int):
  val r =
    val a = 1
    val b = 2
    val c = 3
    a + b + c + longName

@main def main(): Unit =
  println(C(10).r)
