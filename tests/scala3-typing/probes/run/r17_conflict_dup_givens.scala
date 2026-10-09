trait Special[A]:
  def name: String
object syntax:
  given Special[Option[Long]] with
    def name = "long"
  given [A] => Special[Option[A]] with
    def name = "generic"
@main def run(): Unit =
  import syntax.given
  println(summon[Special[Option[Long]]].name)
  println(summon[Special[Option[String]]].name)
