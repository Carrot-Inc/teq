package shape

@jsImport("node:os", "platform")
def platform(): String

def unusedHelper(path: String): String = platform() + path

val unusedTop: Int =
  println("unusedTop initialised")
  1
