// An object read in a try's body and three times after it: its initializer runs where it is first read after
// the try, after the argument evaluated before it, since the body's read may not run (the exception).
object VeryLongModuleNameForInitialization:
  println("init")
  var n = 1

def fail(): Unit = throw new RuntimeException()
def before(): Int = { println("before"); 0 }
def use(a: Int, b: Int, c: Int, d: Int): Unit =
  println(a + b + c + d)

@main def test(): Unit =
  try
    fail()
    println(VeryLongModuleNameForInitialization.n)
  catch case _: RuntimeException => ()
  use(before(), VeryLongModuleNameForInitialization.n,
      VeryLongModuleNameForInitialization.n,
      VeryLongModuleNameForInitialization.n)
