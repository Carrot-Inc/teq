trait Special[A]
object syntax:
  given Special[Option[Long]] = ???
  given [A] => Special[Option[A]] = ???
@main def run(): Unit = println(1)
