package filechecks

override def toString: String = "holder"
override def hashCode(): Int = 17
override def equals(other: Any): Boolean = false

def ordinary: Int = 2

@main def runFileOverrides(): Unit =
  println(toString())
  println(hashCode())
  println(equals("other"))
  println(ordinary)
