import scala.collection.mutable.ArrayBuffer

val log = ArrayBuffer.empty[String]

final class Connection(val name: String):
  private var open = true
  def query(sql: String): List[String] =
    if !open then throw new IllegalStateException(s"$name is closed")
    if sql.startsWith("bad") then throw new IllegalArgumentException(s"cannot run '$sql'")
    List(s"$name:$sql")
  def close(): Unit =
    open = false
    log += s"closed $name"

def withConnection[A](name: String)(use: Connection => A): A =
  val c = new Connection(name)
  try use(c)
  finally c.close()

def runAll(sqls: List[String]): Either[String, List[String]] =
  try Right(withConnection("db")(c => sqls.flatMap(c.query)))
  catch
    case e: IllegalArgumentException => Left(e.getMessage)

class Retry(val attempts: Int) extends RuntimeException(s"gave up after $attempts attempts")

def retry[A](times: Int)(body: Int => A): A =
  var attempt = 1
  var result: Option[A] = None
  while result.isEmpty do
    try result = Some(body(attempt))
    catch
      case e: IllegalStateException if attempt < times =>
        log += s"attempt $attempt failed: ${e.getMessage}"
        attempt += 1
      case e: IllegalStateException => throw new Retry(attempt)
  result.get

def escaping(): Int =
  val c = new Connection("temp")
  try
    c.query("bad idea")
    1
  finally
    log += "finally before propagation"

@main def main(): Unit =
  println(runAll(List("select 1", "select 2")))
  println(runAll(List("select 1", "bad query", "select 3")))
  println(log.toList)
  log.clear()
  println(retry(3)(n => if n < 3 then throw new IllegalStateException(s"flaky $n") else s"ok on $n"))
  println(log.toList)
  log.clear()
  println(try retry(2)(n => throw new IllegalStateException("never")) catch { case r: Retry => r.getMessage })
  println(log.toList)
  log.clear()
  println(try escaping() catch { case e: IllegalArgumentException => "escaped: " + e.getMessage })
  println(log.toList)
  log.clear()
  val c = withConnection("outer")(c => c)
  println(try c.query("x") catch { case e: IllegalStateException => List(e.getMessage) })
  val results =
    for sql <- List("a", "bad b", "c")
    yield try withConnection("loop")(_.query(sql)).head catch { case e: IllegalArgumentException => "skipped" }
  println(results)
  println(log.toList)
  def nestedFinally(): String =
    val steps = ArrayBuffer.empty[String]
    try
      try
        steps += "inner body"
        throw new RuntimeException("from inner")
      finally
        steps += "inner finally"
    catch
      case e: RuntimeException =>
        steps += "outer catch " + e.getMessage
    finally
      steps += "outer finally"
    steps.mkString(", ")
  println(nestedFinally())
  def finallyOverrides(): Int =
    var n = 0
    try
      n = 1
      n
    finally
      n = 2
  println(finallyOverrides())
