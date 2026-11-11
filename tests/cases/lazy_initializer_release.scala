//> using platform jvm
// CollectNullableFields keeps only a private field read by one lazy initializer.
// A successful initialization releases it; a failed one, a second reader and
// a lambda returned by the initializer retain it.
class Once(data: Object):
  lazy val value: Int = data.hashCode()
class Retry(data: Object):
  var attempts = 0
  lazy val value: Int =
    attempts += 1
    if attempts == 1 then throw new RuntimeException("retry")
    data.hashCode()
class Shared(data: Object):
  lazy val value: Int = data.hashCode()
  def again: Int = data.hashCode()
class Escaped(data: Object):
  lazy val value: () => Int = () => data.hashCode()
class Two(data: Object):
  lazy val first: Int = data.hashCode()
  lazy val second: Int = data.hashCode()
object Main:
  def retained(instance: Object): Boolean =
    val field = instance.getClass.getDeclaredField("data")
    field.setAccessible(true)
    field.get(instance) != null
  def main(args: Array[String]): Unit =
    val data = new Object:
      override def hashCode(): Int = 42
    val once = new Once(data)
    println(retained(once))
    println(once.value)
    println(retained(once))
    val retry = new Retry(data)
    try retry.value catch case _: RuntimeException => ()
    println(retained(retry))
    println(retry.value)
    println(retained(retry))
    val shared = new Shared(data)
    println(shared.value + shared.again)
    println(retained(shared))
    val escaped = new Escaped(data)
    val read = escaped.value
    println(read())
    println(retained(escaped))
    val two = new Two(data)
    println(two.first + two.second)
    println(retained(two))
