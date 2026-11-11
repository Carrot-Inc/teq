object Trace:
  def mark(label: String, value: Int): Int =
    println(label)
    value
final case class Entry(n: Int = Trace.mark("default", 5))(val m: Int = Trace.mark("later", n + 1))
object Entry:
  println("init")
def receiver: Entry.type =
  println("receiver")
  Entry
@main def run(): Unit =
  println(receiver(n = Trace.mark("arg", 7))().m)
  println(Entry()().m)
