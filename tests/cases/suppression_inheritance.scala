// The second code pass's `suppression_inheritance` program.
class Quiet(enabled: Boolean) extends RuntimeException("q", null, enabled, false)
@main def run(): Unit =
  for enabled <- List(false, true) do
    val q = new Quiet(enabled)
    q.addSuppressed(new Exception("first"))
    q.addSuppressed(new Exception("second"))
    val copy = q.getSuppressed()
    println(copy.map(_.getMessage()).mkString(","))
    if copy.length > 0 then copy(0) = new Exception("changed")
    println(q.getSuppressed().map(_.getMessage()).mkString(","))
    try q.addSuppressed(null) catch case _: NullPointerException => println("null")
