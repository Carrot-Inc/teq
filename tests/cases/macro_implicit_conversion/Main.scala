package mic

def join(a: Chunk, b: Chunk): String = (a.segments ++ b.segments).mkString("|")

@main def run(): Unit =
  val literal: Chunk = "out"
  println(literal)
  val dynamic = "in" + "put"
  println(join("a/b", dynamic))
